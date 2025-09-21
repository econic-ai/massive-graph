//! Configuration for Massive Graph Database
//! 
//! This module handles configuration settings focused on storage and essential services.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use crate::{log_info, log_warn};


/// Available storage backend types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageType {
    /// In-memory storage using SimpleDocumentStorage
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
    
    /// QUIC ingress configuration
    pub quic: QuicConfig,
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

/// QUIC service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuicConfig {
    /// Enable QUIC service
    pub enabled: bool,
    
    /// Bind address for QUIC server
    pub bind_address: SocketAddr,
    
    /// Number of shards for document-level sharding
    pub shard_count: u16,
    
    /// Number of worker threads per shard
    pub workers_per_shard: usize,
    
    /// Maximum connections
    pub max_connections: usize,
    
    /// Queue size between ingress and validation
    pub queue_size: usize,
    
    /// Certificate path for TLS
    pub cert_path: Option<String>,
    
    /// Key path for TLS
    pub key_path: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            metrics: MetricsConfig::default(),
            quic: QuicConfig::default(),
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

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_address: "0.0.0.0:4433".parse().unwrap(),
            shard_count: 16,
            workers_per_shard: 2,
            max_connections: 10_000,
            queue_size: 100_000,
            cert_path: None,
            key_path: None,
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
                    log_info!("Loaded configuration from: {}", path);
                    config
                }
                Err(e) => {
                    log_warn!("Failed to load config from {}: {}. Using defaults.", path, e);
                    Config::default()
                }
            }
        }
        None => {
            log_info!("No config file specified, using defaults");
            Config::default()
        }
    }
}
