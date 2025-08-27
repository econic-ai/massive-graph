//! Metrics collection and monitoring for Massive Graph Database
//! 
//! This module provides high-performance metrics collection using Prometheus,
//! optimized for minimal overhead during normal operations.

use crate::types::Result;
use once_cell::sync::Lazy;
use prometheus::{
    register_gauge, register_histogram, register_int_counter,
    register_int_gauge, Gauge, Histogram, IntCounter, IntGauge, Registry,
};
use std::time::Instant;

/// Global metrics registry
static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

/// Operation counters for tracking graph database operations
pub struct OperationMetrics {
    /// Total number of nodes created
    pub nodes_created: IntCounter,
    /// Total number of nodes updated
    pub nodes_updated: IntCounter,
    /// Total number of nodes deleted
    pub nodes_deleted: IntCounter,
    /// Total number of edges created
    pub edges_created: IntCounter,
    /// Total number of edges updated
    pub edges_updated: IntCounter,
    /// Total number of edges deleted
    pub edges_deleted: IntCounter,
    /// Total number of delta operations processed successfully
    pub deltas_processed: IntCounter,
    /// Total number of failed delta operations
    pub deltas_failed: IntCounter,
}

/// Performance metrics for monitoring system resource usage
pub struct PerformanceMetrics {
    /// Histogram of operation durations in seconds
    pub operation_duration: Histogram,
    /// Current memory usage in bytes
    pub memory_usage: Gauge,
    /// Number of active network connections
    pub active_connections: IntGauge,
    /// Current size of the operation queue
    pub queue_size: IntGauge,
    /// Current CPU usage percentage (0-100)
    pub cpu_usage: Gauge,
}

/// Storage metrics for monitoring disk and cache performance
pub struct StorageMetrics {
    /// Current disk usage in bytes
    pub disk_usage: Gauge,
    /// Total number of disk read operations
    pub disk_reads: IntCounter,
    /// Total number of disk write operations
    pub disk_writes: IntCounter,
    /// Total number of cache hits
    pub cache_hits: IntCounter,
    /// Total number of cache misses
    pub cache_misses: IntCounter,
}

/// Network metrics for monitoring communication and data transfer
pub struct NetworkMetrics {
    /// Total bytes sent over the network
    pub bytes_sent: IntCounter,
    /// Total bytes received over the network
    pub bytes_received: IntCounter,
    /// Total number of connections accepted
    pub connections_accepted: IntCounter,
    /// Total number of connections closed
    pub connections_closed: IntCounter,
    /// Total number of messages sent
    pub messages_sent: IntCounter,
    /// Total number of messages received
    pub messages_received: IntCounter,
}

/// Centralized metrics collection for all system components
pub struct Metrics {
    /// Graph operation metrics (CRUD operations on nodes/edges)
    pub operations: OperationMetrics,
    /// System performance metrics (CPU, memory, connections)
    pub performance: PerformanceMetrics,
    /// Storage and caching metrics
    pub storage: StorageMetrics,
    /// Network communication metrics
    pub network: NetworkMetrics,
}

impl Metrics {
    /// Create new metrics instance
    pub fn new() -> Result<Self> {
        Ok(Self {
            operations: OperationMetrics::new()?,
            performance: PerformanceMetrics::new()?,
            storage: StorageMetrics::new()?,
            network: NetworkMetrics::new()?,
        })
    }

    /// Get the global metrics instance
    pub fn global() -> &'static Metrics {
        static INSTANCE: Lazy<Metrics> = Lazy::new(|| {
            Metrics::new().expect("Failed to initialize metrics")
        });
        &INSTANCE
    }
}

impl OperationMetrics {
    /// Create a new OperationMetrics instance with registered Prometheus counters
    fn new() -> Result<Self> {
        Ok(Self {
            nodes_created: register_int_counter!(
                "mg_nodes_created_total",
                "Total number of nodes created"
            )?,
            nodes_updated: register_int_counter!(
                "mg_nodes_updated_total",
                "Total number of nodes updated"
            )?,
            nodes_deleted: register_int_counter!(
                "mg_nodes_deleted_total",
                "Total number of nodes deleted"
            )?,
            edges_created: register_int_counter!(
                "mg_edges_created_total",
                "Total number of edges created"
            )?,
            edges_updated: register_int_counter!(
                "mg_edges_updated_total",
                "Total number of edges updated"
            )?,
            edges_deleted: register_int_counter!(
                "mg_edges_deleted_total",
                "Total number of edges deleted"
            )?,
            deltas_processed: register_int_counter!(
                "mg_deltas_processed_total",
                "Total number of deltas processed"
            )?,
            deltas_failed: register_int_counter!(
                "mg_deltas_failed_total",
                "Total number of failed delta operations"
            )?,
        })
    }
}

impl PerformanceMetrics {
    /// Create a new PerformanceMetrics instance with registered Prometheus metrics
    fn new() -> Result<Self> {
        Ok(Self {
            operation_duration: register_histogram!(
                "mg_operation_duration_seconds",
                "Duration of database operations in seconds",
                vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
            )?,
            memory_usage: register_gauge!(
                "mg_memory_usage_bytes",
                "Current memory usage in bytes"
            )?,
            active_connections: register_int_gauge!(
                "mg_active_connections",
                "Number of active connections"
            )?,
            queue_size: register_int_gauge!(
                "mg_queue_size",
                "Current size of operation queue"
            )?,
            cpu_usage: register_gauge!(
                "mg_cpu_usage_percent",
                "Current CPU usage percentage"
            )?,
        })
    }
}

impl StorageMetrics {
    /// Create a new StorageMetrics instance with registered Prometheus metrics
    fn new() -> Result<Self> {
        Ok(Self {
            disk_usage: register_gauge!(
                "mg_disk_usage_bytes",
                "Current disk usage in bytes"
            )?,
            disk_reads: register_int_counter!(
                "mg_disk_reads_total",
                "Total number of disk read operations"
            )?,
            disk_writes: register_int_counter!(
                "mg_disk_writes_total",
                "Total number of disk write operations"
            )?,
            cache_hits: register_int_counter!(
                "mg_cache_hits_total",
                "Total number of cache hits"
            )?,
            cache_misses: register_int_counter!(
                "mg_cache_misses_total",
                "Total number of cache misses"
            )?,
        })
    }
}

impl NetworkMetrics {
    /// Create a new NetworkMetrics instance with registered Prometheus metrics
    fn new() -> Result<Self> {
        Ok(Self {
            bytes_sent: register_int_counter!(
                "mg_network_bytes_sent_total",
                "Total bytes sent over network"
            )?,
            bytes_received: register_int_counter!(
                "mg_network_bytes_received_total",
                "Total bytes received over network"
            )?,
            connections_accepted: register_int_counter!(
                "mg_connections_accepted_total",
                "Total connections accepted"
            )?,
            connections_closed: register_int_counter!(
                "mg_connections_closed_total",
                "Total connections closed"
            )?,
            messages_sent: register_int_counter!(
                "mg_messages_sent_total",
                "Total messages sent"
            )?,
            messages_received: register_int_counter!(
                "mg_messages_received_total",
                "Total messages received"
            )?,
        })
    }
}

/// Timer for measuring operation duration with automatic histogram recording
pub struct Timer {
    /// Start time of the operation
    start: Instant,
    /// Histogram to record the duration when finished
    histogram: Histogram,
}

impl Timer {
    /// Start a new timer
    pub fn start(histogram: Histogram) -> Self {
        Self {
            start: Instant::now(),
            histogram,
        }
    }

    /// Record the elapsed time and consume the timer
    pub fn finish(self) {
        let duration = self.start.elapsed();
        self.histogram.observe(duration.as_secs_f64());
    }
}

/// Convenience macro for timing operations and automatically recording duration
/// 
/// # Examples
/// ```
/// use massive_graph::time_operation;
/// let metrics = Metrics::global();
/// let result = time_operation!(metrics.performance.operation_duration, {
///     // Your operation here
///     expensive_computation()
/// });
/// ```
#[macro_export]
macro_rules! time_operation {
    ($metric:expr, $body:expr) => {{
        let timer = $crate::metrics::Timer::start($metric.clone());
        let result = $body;
        timer.finish();
        result
    }};
}

/// Initialize the metrics registry by creating the global metrics instance
/// 
/// This function should be called once during application startup to ensure
/// all metrics are properly registered with Prometheus.
pub fn init_registry() {
    // Initialize global metrics to register them
    let _ = Metrics::global();
}

/// Get the Prometheus registry for serving metrics to monitoring systems
/// 
/// This registry contains all registered metrics and can be used with
/// Prometheus HTTP endpoints or other metric collection systems.
pub fn registry() -> &'static Registry {
    &REGISTRY
}

/// Collect and return all metrics as a Prometheus-formatted string
/// 
/// This function gathers all registered metrics and formats them according
/// to the Prometheus exposition format for HTTP endpoints.
pub fn collect_metrics() -> String {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = registry().gather();
    encoder.encode_to_string(&metric_families).unwrap_or_default()
}

/// Update system metrics periodically with current resource usage
/// 
/// This function should be called regularly (e.g., every 5-10 seconds) to
/// keep system metrics current. It updates memory usage, CPU usage, and disk usage.
pub async fn update_system_metrics() {
    let metrics = Metrics::global();
    
    // Update memory usage
    if let Ok(memory) = get_memory_usage() {
        metrics.performance.memory_usage.set(memory as f64);
    }
    
    // Update CPU usage
    if let Ok(cpu) = get_cpu_usage().await {
        metrics.performance.cpu_usage.set(cpu);
    }
    
    // Update disk usage
    if let Ok(disk) = get_disk_usage() {
        metrics.storage.disk_usage.set(disk as f64);
    }
}

/// Get current memory usage in bytes
/// 
/// # Note
/// This is a simplified implementation. In production, use a proper system
/// monitoring crate like `sysinfo` or `procfs` for accurate measurements.
fn get_memory_usage() -> Result<usize> {
    // Simplified implementation - in production use a proper system monitoring crate
    Ok(0)
}

/// Get current CPU usage percentage (0.0 to 100.0)
/// 
/// # Note
/// This is a simplified implementation. In production, use a proper system
/// monitoring crate like `sysinfo` for accurate CPU measurements.
async fn get_cpu_usage() -> Result<f64> {
    // Simplified implementation - in production use a proper system monitoring crate
    Ok(0.0)
}

/// Get current disk usage in bytes
/// 
/// # Note
/// This is a simplified implementation. In production, use a proper system
/// monitoring crate like `sysinfo` or filesystem APIs for accurate measurements.
fn get_disk_usage() -> Result<usize> {
    // Simplified implementation - in production use a proper system monitoring crate
    Ok(0)
} 