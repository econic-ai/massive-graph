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
    
    /// QUIC ingress configuration (server only)
    #[serde(default)]
    pub quic: Option<QuicConfig>,
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
    #[serde(default = "default_quic_enabled")]
    pub enabled: bool,
    
    /// Bind address for QUIC server
    #[serde(default = "default_quic_bind_address")]
    pub bind_address: SocketAddr,
    
    /// Number of shards for document-level sharding
    #[serde(default = "default_shard_count")]
    pub shard_count: u16,
    
    /// Number of worker threads per shard
    #[serde(default = "default_workers_per_shard")]
    pub workers_per_shard: usize,
    
    /// Maximum connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    
    /// Queue size between ingress and validation
    #[serde(default = "default_queue_size")]
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
            quic: None,
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
            enabled: default_quic_enabled(),
            bind_address: default_quic_bind_address(),
            shard_count: default_shard_count(),
            workers_per_shard: default_workers_per_shard(),
            max_connections: default_max_connections(),
            queue_size: default_queue_size(),
            cert_path: None,
            key_path: None,
        }
    }
}

// Default value functions for serde
fn default_quic_enabled() -> bool { false }
fn default_quic_bind_address() -> SocketAddr { "0.0.0.0:4433".parse().unwrap() }
fn default_shard_count() -> u16 { 16 }
fn default_workers_per_shard() -> usize { 2 }
fn default_max_connections() -> usize { 10_000 }
fn default_queue_size() -> usize { 100_000 }

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
