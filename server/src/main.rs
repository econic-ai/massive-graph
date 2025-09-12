//! Massive Graph Database Server
//! 
//! High-performance real-time graph database - HTTP POC
use clap::{Arg, Command};
use tokio::signal;
use massive_graph_core::core::{config, factory::create_app_state};
use massive_graph_core::{log_info, log_warn};


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
    let quic_app_state = configured_app_state.clone();
    let api_app_state = configured_app_state.clone();
    log_info!("AppState created successfully");
    
    // Start the HTTP server
    let api_handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
        massive_graph_server::api::api_server::start_api_server(api_app_state)
            .await
            .expect("HTTP server failed")
    });
    
    // Start QUIC service if enabled
    let quic_handle: tokio::task::JoinHandle<()> = tokio::spawn(async move {
        massive_graph_server::quic::run_quic_service(quic_app_state)
            .await
            .expect("QUIC service failed")
    });
    
    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            log_warn!("Received shutdown signal");
        }
        _ = api_handle => {
            log_warn!("HTTP server terminated unexpectedly");
        }
        _ = quic_handle => {
            log_warn!("QUIC server terminated unexpectedly");
        }
    }

    log_info!("Shutdown complete");
    Ok(())
}
