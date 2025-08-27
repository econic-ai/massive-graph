//! Configuration for Massive Graph Database
//! 
//! This module handles configuration settings focused on storage and essential services.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;


/// Available storage backend types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    /// In-memory storage using SimpleStorage
    Simple,
    /// In-memory storage using ZeroCopyStorage  
    ZeroCopy,
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// Metrics configuration
    pub metrics: MetricsConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// HTTP server bind address
    pub http_addr: SocketAddr,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Storage backend type
    pub storage_type: StorageType,
    
    /// Data directory path (for future disk storage)
    pub data_dir: PathBuf,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable Prometheus metrics
    pub enable_prometheus: bool,
    
    /// Metrics server bind address
    pub metrics_addr: SocketAddr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            metrics: MetricsConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            http_addr: "0.0.0.0:8080".parse().unwrap(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            storage_type: StorageType::Simple,
            data_dir: PathBuf::from("./data"),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_prometheus: false,
            metrics_addr: "0.0.0.0:9090".parse().unwrap(),
        }
    }
}

/// Load configuration from file
pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let config_str = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}

/// Load configuration from file or use defaults
pub fn load_config_or_default(path: Option<&str>) -> Config {
    match path {
        Some(path) => {
            match load_config(path) {
                Ok(config) => {
                    tracing::info!("Loaded configuration from: {}", path);
                    config
                }
                Err(e) => {
                    tracing::warn!("Failed to load config from {}: {}. Using defaults.", path, e);
                    Config::default()
                }
            }
        }
        None => {
            tracing::info!("No config file specified, using defaults");
            Config::default()
        }
    }
}
