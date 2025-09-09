//! Massive Graph Database Server
//! 
//! High-performance real-time graph database - HTTP POC

use clap::{Arg, Command};
use tokio::signal;
use massive_graph_core::core::{config, factory::create_app_state};
use massive_graph_core::{log_info, log_warn, log_error};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let matches = Command::new("massive-graph")
        .version("0.1.0")
        .about("Massive Graph Database POC")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
        )
        .get_matches();

    // Initialize logging
    tracing_subscriber::fmt::init();

    log_info!("Starting Massive Graph Database POC");

    // Load configuration
    let config_path = matches.get_one::<String>("config").map(|s| s.as_str());
    let config = config::load_config_or_default(config_path);
    
    // Create AppState using factory pattern
    let configured_app_state = create_app_state(config)?;
    log_info!("AppState created successfully");
    
    // Extract config for QUIC
    let quic_config = configured_app_state.config().quic.clone();
    
    // Start the HTTP server
    let http_app_state = configured_app_state;
    let server_handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
        massive_graph_server::api::api_server::start_api_server(http_app_state)
            .await
            .expect("HTTP server failed")
    });
    
    // Start QUIC service if enabled
    let quic_handle = if let Some(quic_cfg) = quic_config {
        if quic_cfg.enabled {
            log_info!("Starting QUIC ingress service");
            Some(tokio::spawn(async move {
                if let Err(e) = massive_graph_server::quic::run_quic_service(quic_cfg).await {
                    log_error!("QUIC service failed: {}", e);
                }
            }))
        } else {
            None
        }
    } else {
        None
    };
    
    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            log_warn!("Received shutdown signal");
        }
        _ = server_handle => {
            log_warn!("HTTP server terminated unexpectedly");
        }
        _ = async {
            if let Some(handle) = quic_handle {
                handle.await.ok();
            }
        } => {
            log_warn!("QUIC server terminated unexpectedly");
        }
    }

    log_info!("Shutdown complete");
    Ok(())
}
