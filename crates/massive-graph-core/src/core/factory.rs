//! Application Factory
//! 
//! This module provides factory functions for creating and initializing the AppState
//! with all required services based on configuration.

use std::sync::Arc;
use crate::core::app_state::AppState;
use crate::core::config::{Config};
use crate::comms::network::Network;
use crate::log_info;
use crate::storage::Store;


/// Create AppState based on configuration (for server use)
pub fn create_app_state(config: Config) -> Arc<AppState> {
    log_info!("Creating AppState with storage type: {:?}", config.storage.storage_type);
    log_info!("Creating AppState with ZeroCopyStorage");
    
    log_info!("Initializing ZeroCopyStore");
    let store = Arc::new(Store::new());
    log_info!("ZeroCopyStore initialized successfully");
    
    log_info!("Initializing Network");
    let network = Network::new();
    log_info!("Network initialized successfully");
    
    let app_state = AppState::new(
        store,
        config,
        network,
    );
    
    log_info!("AppState with ZeroCopyStorage created successfully");
    Arc::new(app_state)
}
