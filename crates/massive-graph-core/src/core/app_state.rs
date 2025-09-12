//! Cross-platform application state
//! 
//! Simple AppState that works for both server and browser environments.

// use std::sync::Arc;
// use crate::comms::ConnectionManager;
// use crate::core::AppConfig;

/// Shared application state for WebRTC POC
// pub struct AppState {
//     /// Connection manager for WebRTC connections
//     pub connection_manager: Arc<ConnectionManager>,
    
//     /// Application configuration
//     pub config: AppConfig,
// }

// impl AppState {
//     /// Create a new AppState with the given configuration
//     pub fn new(config: AppConfig) -> Self {
//         let connection_manager = Arc::new(ConnectionManager::new(
//             config.connection_id.clone(),
//             config.is_server,
//         ));
        
//         Self {
//             connection_manager,
//             config,
//         }
//     }
    
//     /// Get our connection ID
//     pub fn connection_id(&self) -> &crate::types::ConnectionId {
//         &self.config.connection_id
//     }
    
//     /// Check if we're running on the server
//     pub fn is_server(&self) -> bool {
//         self.config.is_server
//     }
// }

use std::sync::Arc;
use crate::comms::network::Network;
use crate::core::config::Config;
use crate::storage::{Store, StorageImpl};

/// Central application state holding all services and components
pub struct AppState<S: StorageImpl>  {
    /// Storage system - configured Store instance
    pub store: Arc<Store<S>>,
    
    /// Application configuration
    pub config: Config,
    
    /// Network with Connection manager
    pub network: Network,

    /// TODO: Replace with actual implementations (DO NOT REMOVE)
    pub metrics: MetricsServiceStub,
    
    /// TODO: implement structured logging (DO NOT REMOVE)
    pub logger: LoggerServiceStub, 
    
    /// TODO: implement system health tracking (DO NOT REMOVE)
    pub health_monitor: HealthMonitorStub, 
    
    /// TODO: implement auth and authorization (DO NOT REMOVE)
    pub security: SecurityContextStub, 
    
    /// TODO: implement document and query caching (DO NOT REMOVE)
    pub cache: CacheManagerStub, 
    
    /// TODO: implement background task scheduling (DO NOT REMOVE)
    pub scheduler: TaskSchedulerStub, 
    
    /// TODO: implement system-wide event notifications (DO NOT REMOVE)
    pub event_bus: EventBusStub, 
    
}

// Manual Clone implementation for AppState that doesn't require S: Clone
// Since storage is held in an Arc, we only need to clone the Arc (increment reference count)
impl<S: StorageImpl> Clone for AppState<S> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),      // Only clones Arc, not the storage itself
            config: self.config.clone(),
            network: self.network.clone(),
            metrics: self.metrics.clone(),
            logger: self.logger.clone(),
            health_monitor: self.health_monitor.clone(),
            security: self.security.clone(),
            cache: self.cache.clone(),
            scheduler: self.scheduler.clone(),
            event_bus: self.event_bus.clone(),
        }
    }
}

// Service stubs - TODO: Replace with actual implementations (DO NOT REMOVE)

/// Metrics service stub - TODO: implement Prometheus metrics collection (DO NOT REMOVE)
#[derive(Clone)]
pub struct MetricsServiceStub;

/// Logger service stub - TODO: implement structured logging (DO NOT REMOVE)
#[derive(Clone)]
pub struct LoggerServiceStub;

/// Health monitor stub - TODO: implement system health tracking (DO NOT REMOVE)
#[derive(Clone)]
pub struct HealthMonitorStub;

/// Security context stub - TODO: implement auth and authorization (DO NOT REMOVE)
#[derive(Clone)]
pub struct SecurityContextStub;

/// Cache manager stub - TODO: implement document and query caching (DO NOT REMOVE)
#[derive(Clone)]
pub struct CacheManagerStub;

/// Task scheduler stub - TODO: implement background task scheduling (DO NOT REMOVE)
#[derive(Clone)]
pub struct TaskSchedulerStub;

/// Event bus stub - TODO: implement system-wide event notifications (DO NOT REMOVE)
#[derive(Clone)]
pub struct EventBusStub;

impl<S: StorageImpl> AppState<S> {
    /// Create a new AppState with the given configuration
    /// This is called by the factory after all services are initialized
    pub fn new(
        store: Arc<Store<S>>,
        config: Config,
        network: Network,
    ) -> Self {
        Self {
            store,
            config,
            network,
            metrics: MetricsServiceStub,
            logger: LoggerServiceStub,
            health_monitor: HealthMonitorStub,
            security: SecurityContextStub,
            cache: CacheManagerStub,
            scheduler: TaskSchedulerStub,
            event_bus: EventBusStub,
        }
    }
}
