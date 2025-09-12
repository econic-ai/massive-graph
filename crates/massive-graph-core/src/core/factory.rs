//! Application Factory
//! 
//! This module provides factory functions for creating and initializing the AppState
//! with all required services based on configuration.

use std::sync::Arc;
use crate::core::app_state::AppState;
use crate::core::config::{Config, QuicConfig, StorageType};
use crate::storage::{SimpleStore, ZeroCopyStore, SimpleStorage, ZeroCopyStorage};
use crate::comms::network::Network;
use crate::log_info;

/// AppState factory errors
#[derive(Debug)]
pub enum AppStateFactoryError {
    /// Unsupported storage type
    UnsupportedStorageType(StorageType),
    /// Storage initialization failed
    StorageInitializationFailed(String),
    /// Configuration error
    ConfigError(String),
}

impl std::fmt::Display for AppStateFactoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppStateFactoryError::UnsupportedStorageType(storage_type) => {
                write!(f, "Unsupported storage type: {:?}", storage_type)
            }
            AppStateFactoryError::StorageInitializationFailed(msg) => {
                write!(f, "Storage initialization failed: {}", msg)
            }
            AppStateFactoryError::ConfigError(msg) => {
                write!(f, "Configuration error: {}", msg)
            }
        }
    }
}

impl std::error::Error for AppStateFactoryError {}

/// Create AppState from configuration
/// 
/// This factory function initializes all services based on the provided configuration.
/// It follows the factory pattern to ensure clean separation of concerns.
/// 
/// # Arguments
/// 
/// * `config` - Application configuration
/// 
/// # Returns
/// 
/// * `Ok(ConfiguredAppState)` - Successfully created and initialized AppState with concrete storage type
/// * `Err(AppStateFactoryError)` - Configuration or initialization error

/// Enum to hold different AppState configurations
pub enum ConfiguredAppState {
    /// Configuration using SimpleStorage backend
    Simple {
        /// The application state with SimpleStorage
        app_state: Arc<AppState<SimpleStorage>>,
        /// The storage instance for server initialization
        storage: Arc<SimpleStore>,
    },
    /// Configuration using ZeroCopyStorage backend
    ZeroCopy {
        /// The application state with ZeroCopyStorage
        app_state: Arc<AppState<ZeroCopyStorage>>,
        /// The storage instance for server initialization
        storage: Arc<ZeroCopyStore>,
    },
}

impl ConfiguredAppState {
    /// Get the HTTP address from config
    pub fn http_addr(&self) -> std::net::SocketAddr {
        match self {
            ConfiguredAppState::Simple { app_state, .. } => app_state.config.server.http_addr,
            ConfiguredAppState::ZeroCopy { app_state, .. } => app_state.config.server.http_addr,
        }
    }
    
    /// Get the configuration
    pub fn quic_config(&self) -> QuicConfig {
        match self {
            ConfiguredAppState::Simple { app_state, .. } => app_state.config.quic.clone(),
            ConfiguredAppState::ZeroCopy { app_state, .. } => app_state.config.quic.clone(),
        }
    }
    
}

impl Clone for ConfiguredAppState {
    fn clone(&self) -> Self {
        match self {
            ConfiguredAppState::Simple { app_state, .. } => ConfiguredAppState::Simple { app_state: app_state.clone(), storage: app_state.store.clone() },
            ConfiguredAppState::ZeroCopy { app_state, .. } => ConfiguredAppState::ZeroCopy { app_state: app_state.clone(), storage: app_state.store.clone() },
        }
    }
}

/// Create AppState with SimpleStorage backend
pub fn create_app_state_with_simple(config: Config) -> AppState<SimpleStorage> {
    log_info!("Creating AppState with SimpleStorage");
    
    log_info!("Initializing SimpleStore");
    let store = SimpleStore::new(|| SimpleStorage::new());
    let storage = Arc::new(store);
    log_info!("SimpleStore initialized successfully");
    
    log_info!("Initializing Network");
    let network = Network::new();
    log_info!("Network initialized successfully");
    
    let app_state = AppState::new(
        storage,
        config,
        network,
    );
    
    log_info!("AppState with SimpleStorage created successfully");
    app_state
}

/// Create AppState with ZeroCopyStorage backend
pub fn create_app_state_with_zerocopy(config: Config) -> AppState<ZeroCopyStorage> {
    log_info!("Creating AppState with ZeroCopyStorage");
    
    log_info!("Initializing ZeroCopyStore");
    let store = ZeroCopyStore::new(|| ZeroCopyStorage::new());
    let storage = Arc::new(store);
    log_info!("ZeroCopyStore initialized successfully");
    
    log_info!("Initializing Network");
    let network = Network::new();
    log_info!("Network initialized successfully");
    
    let app_state = AppState::new(
        storage,
        config,
        network,
    );
    
    log_info!("AppState with ZeroCopyStorage created successfully");
    app_state
}

/// Create AppState based on configuration (for server use)
pub fn create_app_state(config: Config) -> Result<ConfiguredAppState, AppStateFactoryError> {
    log_info!("Creating AppState with storage type: {:?}", config.storage.storage_type);
    
    match config.storage.storage_type {
        StorageType::Simple => {
            let app_state = Arc::new(create_app_state_with_simple(config.clone()));
            let storage = app_state.store.clone();
            Ok(ConfiguredAppState::Simple { app_state, storage })
        }
        StorageType::ZeroCopy => {
            let app_state = Arc::new(create_app_state_with_zerocopy(config.clone()));
            let storage = app_state.store.clone();
            Ok(ConfiguredAppState::ZeroCopy { app_state, storage })
        }
    }
}
