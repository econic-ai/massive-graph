//! Massive Graph Database Server
//! 
//! High-performance real-time graph database optimized for collaborative intelligence.
//! Supports HTTP, WebSocket, and QUIC/WebTransport protocols

use clap::{Arg, Command};
use massive_graph::{core::Config, Result};
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};
use massive_graph::api::start_server;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let matches = Command::new("massive-graph")
        .version(massive_graph::VERSION)
        .about("High-performance real-time graph database.")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
        )
        .arg(
            Arg::new("http-addr")
                .long("http-addr")
                .value_name("ADDR")
                .help("HTTP server bind address")
        )
        .arg(
            Arg::new("ws-addr")
                .long("ws-addr")
                .value_name("ADDR")
                .help("WebSocket server bind address")
        )
        .arg(
            Arg::new("quic-addr")
                .long("quic-addr")
                .value_name("ADDR")
                .help("QUIC server bind address")
        )
        .arg(
            Arg::new("data-dir")
                .long("data-dir")
                .value_name("DIR")
                .help("Data directory path")
        )
        .arg(
            Arg::new("workers")
                .long("workers")
                .value_name("N")
                .help("Number of worker threads")
        )
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .value_name("LEVEL")
                .help("Log level (trace, debug, info, warn, error)")
        )
        .arg(
            Arg::new("storage-type")
                .long("storage-type")
                .value_name("TYPE")
                .help("Storage backend type (memory, disk, distributed)")
        )
        .get_matches();

    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let mut config = if let Some(config_path) = matches.get_one::<String>("config") {
        Config::from_file(config_path)?
    } else {
        Config::load()?
    };

    // Apply CLI overrides
    apply_cli_overrides(&mut config, &matches)?;

    info!("Starting Massive Graph Database v{}", env!("CARGO_PKG_VERSION"));
    // info!("Configuration: {:#?}", config);

    // Validate system requirements
    validate_system_requirements(&config)?;

    // Initialize storage
    let storage = massive_graph::storage::create_storage(&config.storage)
        .map_err(|e| massive_graph::Error::config(format!("Storage initialization failed: {}", e)))?;
    info!("Storage initialized: {:?}", config.storage.storage_type);

    // Wrap storage in Arc for sharing between servers (reads are lock-free)
    let storage = Arc::new(storage);

    // Create shared configuration
    let config = Arc::new(config);

    // Setup graceful shutdown handling
    let shutdown_signal = setup_shutdown_handler();

    // Start all servers concurrently
    let server_handles = start_servers(config.clone(), storage).await?;

    // Wait for shutdown signal
    shutdown_signal.await;
    warn!("Received shutdown signal, initiating graceful shutdown...");

    // Gracefully shutdown all servers
    shutdown_servers(server_handles).await?;

    info!("Shutdown complete");
    Ok(())
}

/// Apply command line argument overrides to configuration
fn apply_cli_overrides(config: &mut Config, matches: &clap::ArgMatches) -> Result<()> {
    if let Some(addr) = matches.get_one::<String>("http-addr") {
        config.server.http_addr = addr.parse()
            .map_err(|e| massive_graph::Error::config(format!("Invalid HTTP address: {}", e)))?;
    }

    if let Some(addr) = matches.get_one::<String>("ws-addr") {
        config.server.ws_addr = addr.parse()
            .map_err(|e| massive_graph::Error::config(format!("Invalid WebSocket address: {}", e)))?;
    }

    if let Some(addr) = matches.get_one::<String>("quic-addr") {
        config.server.quic_addr = addr.parse()
            .map_err(|e| massive_graph::Error::config(format!("Invalid QUIC address: {}", e)))?;
    }

    if let Some(data_dir) = matches.get_one::<String>("data-dir") {
        config.storage.data_dir = data_dir.into();
    }

    if let Some(workers) = matches.get_one::<String>("workers") {
        config.performance.worker_threads = workers.parse()
            .map_err(|e| massive_graph::Error::config(format!("Invalid worker count: {}", e)))?;
    }

    if let Some(level) = matches.get_one::<String>("log-level") {
        config.logging.level = level.clone();
    }

    if let Some(storage_type) = matches.get_one::<String>("storage-type") {
        config.storage.storage_type = match storage_type.as_str() {
            "memory" => massive_graph::core::config::StorageType::Memory,
            "disk" => massive_graph::core::config::StorageType::Disk,
            "distributed" => massive_graph::core::config::StorageType::Distributed,
            _ => return Err(massive_graph::Error::config(
                format!("Invalid storage type: {}. Valid options: memory, disk, distributed", storage_type)
            )),
        };
    }

    Ok(())
}

/// Validate system requirements and configuration
fn validate_system_requirements(config: &Config) -> Result<()> {
    // Check if data directory exists or can be created
    if !config.storage.data_dir.exists() {
        std::fs::create_dir_all(&config.storage.data_dir)
            .map_err(|e| massive_graph::Error::config(
                format!("Cannot create data directory {:?}: {}", config.storage.data_dir, e)
            ))?;
        info!("Created data directory: {:?}", config.storage.data_dir);
    }

    // Check available memory
    let available_memory = get_available_memory();
    if config.storage.max_memory > available_memory {
        warn!(
            "Configured max memory ({} GB) exceeds available memory ({} GB)",
            config.storage.max_memory / (1024 * 1024 * 1024),
            available_memory / (1024 * 1024 * 1024)
        );
    }

    // Check optimal thread count
    let optimal_threads = config.optimal_worker_threads();
    if config.performance.worker_threads != 0 && config.performance.worker_threads != optimal_threads {
        info!(
            "Using {} worker threads (optimal: {})",
            config.performance.worker_threads,
            optimal_threads
        );
    }

    // Validate port availability
    validate_port_availability(config)?;

    Ok(())
}

/// Check if specified ports are available
fn validate_port_availability(config: &Config) -> Result<()> {
    use std::net::TcpListener;

    let ports = [
        ("HTTP", config.server.http_addr),
        ("WebSocket", config.server.ws_addr),
        ("QUIC", config.server.quic_addr),
        ("Metrics", config.metrics.metrics_addr),
    ];

    for (name, addr) in ports {
        TcpListener::bind(addr)
            .map_err(|e| massive_graph::Error::config(
                format!("{} port {} is not available: {}", name, addr.port(), e)
            ))?;
    }

    Ok(())
}

/// Get available system memory in bytes
fn get_available_memory() -> usize {
    // This is a simplified implementation
    // In production, you'd want to use a proper system info crate
    8 * 1024 * 1024 * 1024 // Default to 8GB
}

/// Setup graceful shutdown signal handling
async fn setup_shutdown_handler() -> () {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received terminate signal");
        },
    }
}

/// Server handle for graceful shutdown
struct ServerHandle {
    name: &'static str,
    handle: tokio::task::JoinHandle<Result<()>>,
}

/// Start all servers concurrently
async fn start_servers(config: Arc<Config>, storage: Arc<massive_graph::storage::MemStore>) -> Result<Vec<ServerHandle>> {
    let mut handles = Vec::new();

    // Start HTTP server
    {
        let config = config.clone();
        let storage = storage.clone();
        let handle = tokio::spawn(async move {
            start_http_server(config, storage).await
        });
        handles.push(ServerHandle {
            name: "HTTP",
            handle,
        });
    }

    // Start WebSocket server
    {
        let config = config.clone();
        let handle = tokio::spawn(async move {
            start_websocket_server(config).await
        });
        handles.push(ServerHandle {
            name: "WebSocket",
            handle,
        });
    }

    // Start QUIC server
    {
        let config = config.clone();
        let handle = tokio::spawn(async move {
            start_quic_server(config).await
        });
        handles.push(ServerHandle {
            name: "QUIC",
            handle,
        });
    }

    // Start metrics server if enabled
    if config.metrics.enable_prometheus {
        let config = config.clone();
        let handle = tokio::spawn(async move {
            start_metrics_server(config).await
        });
        handles.push(ServerHandle {
            name: "Metrics",
            handle,
        });
    }

    // Give servers time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    info!("All servers started successfully");
    Ok(handles)
}

/// Start HTTP server
async fn start_http_server(config: Arc<Config>, storage: Arc<massive_graph::storage::MemStore>) -> Result<()> {
    info!("Starting HTTP server on {}", config.server.http_addr);
    
    // Configure server address
    let addr = config.server.http_addr;
    
    // Start the server
    start_server(addr, storage).await
        .map_err(|e| massive_graph::Error::config(format!("HTTP server failed: {}", e)))?;

    Ok(())
}

/// Start WebSocket server
async fn start_websocket_server(config: Arc<Config>) -> Result<()> {
    info!("Starting WebSocket server on {}", config.server.ws_addr);
    
    // TODO: Implement actual WebSocket server
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

/// Start QUIC server
async fn start_quic_server(config: Arc<Config>) -> Result<()> {
    info!("Starting QUIC server on {}", config.server.quic_addr);
    
    // TODO: Implement actual QUIC server
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

/// Start metrics server
async fn start_metrics_server(config: Arc<Config>) -> Result<()> {
    info!("Starting metrics server on {}", config.metrics.metrics_addr);
    
    // TODO: Implement actual metrics server
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
}

/// Gracefully shutdown all servers
async fn shutdown_servers(handles: Vec<ServerHandle>) -> Result<()> {
    info!("Shutting down {} servers...", handles.len());

    // Send shutdown signals to all servers
    for handle in handles {
        info!("Shutting down {} server...", handle.name);
        handle.handle.abort();
        
        // In a real implementation, you'd send a proper shutdown signal
        // and wait for graceful shutdown with a timeout
    }

    info!("All servers shut down");
    Ok(())
}
