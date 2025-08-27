//! Application Factory
//! 
//! This module provides factory functions for creating and initializing the AppState
//! with all required services based on configuration.

use std::sync::Arc;
use crate::core::app_state::AppState;
use crate::core::config::{Config, StorageType};
use crate::storage::{SimpleStore, ZeroCopyStore, SimpleStorage, ZeroCopyStorage};

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
        app_state: AppState<SimpleStorage>,
        /// The storage instance for server initialization
        storage: Arc<SimpleStore>,
    },
    /// Configuration using ZeroCopyStorage backend
    ZeroCopy {
        /// The application state with ZeroCopyStorage
        app_state: AppState<ZeroCopyStorage>,
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
    
    /// Start the server with the appropriate storage type
    pub async fn start_server(self) -> Result<(), Box<dyn std::error::Error>> {
        let http_addr = self.http_addr();
        match self {
            ConfiguredAppState::Simple { storage, .. } => {
                crate::api::server::start_server(http_addr, storage).await
            }
            ConfiguredAppState::ZeroCopy { storage, .. } => {
                crate::api::server::start_server(http_addr, storage).await
            }
        }
    }
}

/// Create AppState based on configuration
pub fn create_app_state(config: Config) -> Result<ConfiguredAppState, AppStateFactoryError> {
    tracing::info!("Creating AppState with storage type: {:?}", config.storage.storage_type);
    
    use crate::core::app_state::{AppState, MetricsServiceStub, LoggerServiceStub, 
        HealthMonitorStub, ConnectionPoolStub, SecurityContextStub, 
        CacheManagerStub, TaskSchedulerStub, EventBusStub};
    
    match config.storage.storage_type {
        StorageType::Simple => {
            tracing::info!("Initializing SimpleStore");
            let store = SimpleStore::new(|| SimpleStorage::new());
            let storage = Arc::new(store);
            tracing::info!("SimpleStore initialized successfully");
            
            let app_state = AppState {
                storage: storage.clone(),
                config: config.clone(),
                metrics: MetricsServiceStub,
                logger: LoggerServiceStub,
                health_monitor: HealthMonitorStub,
                connection_pool: ConnectionPoolStub,
                security: SecurityContextStub,
                cache: CacheManagerStub,
                scheduler: TaskSchedulerStub,
                event_bus: EventBusStub,
            };
            
            tracing::info!("AppState created successfully");
            Ok(ConfiguredAppState::Simple { app_state, storage })
        }
        StorageType::ZeroCopy => {
            tracing::info!("Initializing ZeroCopyStore");
            let store = ZeroCopyStore::new(|| ZeroCopyStorage::new());
            let storage = Arc::new(store);
            tracing::info!("ZeroCopyStore initialized successfully");
            
            let app_state = AppState {
                storage: storage.clone(),
                config: config.clone(),
                metrics: MetricsServiceStub,
                logger: LoggerServiceStub,
                health_monitor: HealthMonitorStub,
                connection_pool: ConnectionPoolStub,
                security: SecurityContextStub,
                cache: CacheManagerStub,
                scheduler: TaskSchedulerStub,
                event_bus: EventBusStub,
            };
            
            tracing::info!("AppState created successfully");
            Ok(ConfiguredAppState::ZeroCopy { app_state, storage })
        }
    }
}
