//! Massive Graph Database Server
//! 
//! High-performance real-time graph database - HTTP POC

use clap::{Arg, Command};
use tokio::signal;
use tracing::{info, warn};
use massive_graph::core::{config, create_app_state};


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

    info!("Starting Massive Graph Database POC");

    // Load configuration
    let config_path = matches.get_one::<String>("config").map(|s| s.as_str());
    let config = config::load_config_or_default(config_path);
    
    // Create AppState using factory pattern
    let configured_app_state = create_app_state(config)?;
    info!("AppState created successfully");
    
    // Start the server with the configured storage type
    let server_handle = tokio::spawn(async move {
        configured_app_state.start_server()
            .await
            .expect("HTTP server failed")
    });
    
    // Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            warn!("Received shutdown signal");
        }
        _ = server_handle => {
            warn!("Server terminated unexpectedly");
        }
    }

    info!("Shutdown complete");
    Ok(())
}
