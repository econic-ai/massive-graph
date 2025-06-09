//! Configuration management for Massive Graph Database
//! 
//! This module handles all configuration settings with performance-optimized defaults.

use crate::core::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    
    /// Storage configuration
    pub storage: StorageConfig,
    
    /// Network configuration
    pub network: NetworkConfig,
    
    /// Performance tuning
    pub performance: PerformanceConfig,
    
    /// Metrics and monitoring
    pub metrics: MetricsConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// HTTP server bind address
    pub http_addr: SocketAddr,
    
    /// WebSocket server bind address  
    pub ws_addr: SocketAddr,
    
    /// QUIC/WebTransport server bind address
    pub quic_addr: SocketAddr,
    
    /// Maximum concurrent connections
    pub max_connections: usize,
    
    /// Request timeout
    pub request_timeout: Duration,
    
    /// Keep-alive timeout
    pub keep_alive: Duration,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Data directory path
    pub data_dir: PathBuf,
    
    /// Maximum memory usage (bytes)
    pub max_memory: usize,
    
    /// Enable memory-mapped files
    pub enable_mmap: bool,
    
    /// Sync to disk interval
    pub sync_interval: Duration,
    
    /// Enable compression
    pub enable_compression: bool,
    
    /// Compression level (1-9)
    pub compression_level: u32,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Maximum message size (bytes)
    pub max_message_size: usize,
    
    /// Connection timeout
    pub connect_timeout: Duration,
    
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    
    /// Maximum retry attempts
    pub max_retries: u32,
    
    /// Enable TCP_NODELAY
    pub tcp_nodelay: bool,
    
    /// Send buffer size
    pub send_buffer_size: usize,
    
    /// Receive buffer size
    pub recv_buffer_size: usize,
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Number of worker threads (0 = auto-detect)
    pub worker_threads: usize,
    
    /// Number of blocking threads
    pub blocking_threads: usize,
    
    /// Maximum batch size for operations
    pub max_batch_size: usize,
    
    /// Delta batch timeout
    pub batch_timeout: Duration,
    
    /// Enable lock-free data structures
    pub enable_lockfree: bool,
    
    /// Memory pool size
    pub memory_pool_size: usize,
    
    /// Enable zero-copy optimizations
    pub enable_zero_copy: bool,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable Prometheus metrics
    pub enable_prometheus: bool,
    
    /// Metrics server bind address
    pub metrics_addr: SocketAddr,
    
    /// Metrics collection interval
    pub collection_interval: Duration,
    
    /// Enable detailed metrics
    pub enable_detailed: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,
    
    /// Log format (json, pretty)
    pub format: String,
    
    /// Enable structured logging
    pub structured: bool,
    
    /// Log file path (None = stdout)
    pub file: Option<PathBuf>,
    
    /// Maximum log file size
    pub max_file_size: usize,
    
    /// Number of log files to keep
    pub max_files: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            storage: StorageConfig::default(),
            network: NetworkConfig::default(),
            performance: PerformanceConfig::default(),
            metrics: MetricsConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            http_addr: "0.0.0.0:8080".parse().unwrap(),
            ws_addr: "0.0.0.0:8081".parse().unwrap(),
            quic_addr: "0.0.0.0:8082".parse().unwrap(),
            max_connections: 10_000,
            request_timeout: Duration::from_secs(30),
            keep_alive: Duration::from_secs(60),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            max_memory: 8 * 1024 * 1024 * 1024, // 8GB
            enable_mmap: true,
            sync_interval: Duration::from_secs(5),
            enable_compression: true,
            compression_level: 6,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            max_message_size: 64 * 1024 * 1024, // 64MB
            connect_timeout: Duration::from_secs(10),
            heartbeat_interval: Duration::from_secs(30),
            max_retries: 3,
            tcp_nodelay: true,
            send_buffer_size: 1024 * 1024, // 1MB
            recv_buffer_size: 1024 * 1024, // 1MB
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            worker_threads: 0, // Auto-detect
            blocking_threads: 512,
            max_batch_size: 1000,
            batch_timeout: Duration::from_millis(10),
            enable_lockfree: true,
            memory_pool_size: 1024 * 1024 * 1024, // 1GB
            enable_zero_copy: true,
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enable_prometheus: true,
            metrics_addr: "0.0.0.0:9090".parse().unwrap(),
            collection_interval: Duration::from_secs(15),
            enable_detailed: false,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "pretty".to_string(),
            structured: false,
            file: None,
            max_file_size: 100 * 1024 * 1024, // 100MB
            max_files: 10,
        }
    }
}

impl Config {
    /// Load configuration from environment variables and config file
    pub fn load() -> Result<Self> {
        let mut config = Config::default();
        
        // Try to load from config file first
        if let Ok(file_config) = Self::from_file("massive-graph.toml") {
            config = file_config;
        }
        
        // Override with environment variables
        config.apply_env_overrides()?;
        
        // Validate configuration
        config.validate()?;
        
        Ok(config)
    }
    
    /// Load configuration from a TOML file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| Error::config(format!("Failed to read config file: {}", e)))?;
            
        toml::from_str(&contents)
            .map_err(|e| Error::config(format!("Failed to parse config file: {}", e)))
    }
    
    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) -> Result<()> {
        use std::env;
        
        // Server overrides
        if let Ok(addr) = env::var("MG_HTTP_ADDR") {
            self.server.http_addr = addr.parse()
                .map_err(|e| Error::config(format!("Invalid HTTP address: {}", e)))?;
        }
        
        if let Ok(addr) = env::var("MG_WS_ADDR") {
            self.server.ws_addr = addr.parse()
                .map_err(|e| Error::config(format!("Invalid WebSocket address: {}", e)))?;
        }
        
        if let Ok(addr) = env::var("MG_QUIC_ADDR") {
            self.server.quic_addr = addr.parse()
                .map_err(|e| Error::config(format!("Invalid QUIC address: {}", e)))?;
        }
        
        if let Ok(max_conn) = env::var("MG_MAX_CONNECTIONS") {
            self.server.max_connections = max_conn.parse()
                .map_err(|e| Error::config(format!("Invalid max connections: {}", e)))?;
        }
        
        // Storage overrides
        if let Ok(data_dir) = env::var("MG_DATA_DIR") {
            self.storage.data_dir = PathBuf::from(data_dir);
        }
        
        if let Ok(max_mem) = env::var("MG_MAX_MEMORY") {
            self.storage.max_memory = max_mem.parse()
                .map_err(|e| Error::config(format!("Invalid max memory: {}", e)))?;
        }
        
        // Performance overrides
        if let Ok(workers) = env::var("MG_WORKER_THREADS") {
            self.performance.worker_threads = workers.parse()
                .map_err(|e| Error::config(format!("Invalid worker threads: {}", e)))?;
        }
        
        // Logging overrides
        if let Ok(level) = env::var("MG_LOG_LEVEL") {
            self.logging.level = level;
        }
        
        if let Ok(format) = env::var("MG_LOG_FORMAT") {
            self.logging.format = format;
        }
        
        Ok(())
    }
    
    /// Validate configuration values
    fn validate(&self) -> Result<()> {
        // Validate ports don't conflict
        let ports = [
            self.server.http_addr.port(),
            self.server.ws_addr.port(),
            self.server.quic_addr.port(),
            self.metrics.metrics_addr.port(),
        ];
        
        for (i, &port1) in ports.iter().enumerate() {
            for &port2 in &ports[i + 1..] {
                if port1 == port2 {
                    return Err(Error::config("Port conflict detected"));
                }
            }
        }
        
        // Validate memory limits
        if self.storage.max_memory < 1024 * 1024 { // Minimum 1MB
            return Err(Error::config("Max memory too small (minimum 1MB)"));
        }
        
        // Validate thread counts
        if self.performance.worker_threads > 1024 {
            return Err(Error::config("Too many worker threads (maximum 1024)"));
        }
        
        // Validate log level
        match self.logging.level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {},
            _ => return Err(Error::config("Invalid log level")),
        }
        
        Ok(())
    }
    
    /// Get optimal number of worker threads
    pub fn optimal_worker_threads(&self) -> usize {
        if self.performance.worker_threads == 0 {
            // Auto-detect: use number of CPU cores
            num_cpus::get().max(1)
        } else {
            self.performance.worker_threads
        }
    }
}

// For TOML parsing
use serde::de::{self, Deserializer, Visitor};
use std::fmt;

// Custom deserializer for Duration from string
fn deserialize_duration<'de, D>(deserializer: D) -> std::result::Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    struct DurationVisitor;
    
    impl<'de> Visitor<'de> for DurationVisitor {
        type Value = Duration;
        
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a duration string like '30s' or '5m'")
        }
        
        fn visit_str<E>(self, value: &str) -> std::result::Result<Duration, E>
        where
            E: de::Error,
        {
            parse_duration(value).map_err(E::custom)
        }
    }
    
    deserializer.deserialize_str(DurationVisitor)
}

// Simple duration parser for common formats
fn parse_duration(s: &str) -> std::result::Result<Duration, String> {
    if s.ends_with("ms") {
        let ms: u64 = s[..s.len() - 2].parse()
            .map_err(|_| "Invalid milliseconds")?;
        Ok(Duration::from_millis(ms))
    } else if s.ends_with('s') {
        let secs: u64 = s[..s.len() - 1].parse()
            .map_err(|_| "Invalid seconds")?;
        Ok(Duration::from_secs(secs))
    } else if s.ends_with('m') {
        let mins: u64 = s[..s.len() - 1].parse()
            .map_err(|_| "Invalid minutes")?;
        Ok(Duration::from_secs(mins * 60))
    } else if s.ends_with('h') {
        let hours: u64 = s[..s.len() - 1].parse()
            .map_err(|_| "Invalid hours")?;
        Ok(Duration::from_secs(hours * 3600))
    } else {
        // Try parsing as raw seconds
        let secs: u64 = s.parse()
            .map_err(|_| "Invalid duration format")?;
        Ok(Duration::from_secs(secs))
    }
} 