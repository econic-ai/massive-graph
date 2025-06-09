//! Metrics collection and monitoring for Massive Graph Database
//! 
//! This module provides high-performance metrics collection using Prometheus,
//! optimized for minimal overhead during normal operations.

use crate::Result;
use once_cell::sync::Lazy;
use prometheus::{
    register_counter, register_gauge, register_histogram, register_int_counter,
    register_int_gauge, Counter, Gauge, Histogram, IntCounter, IntGauge, Registry,
};
use std::time::Instant;

/// Global metrics registry
static REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

/// Operation counters
pub struct OperationMetrics {
    pub nodes_created: IntCounter,
    pub nodes_updated: IntCounter,
    pub nodes_deleted: IntCounter,
    pub edges_created: IntCounter,
    pub edges_updated: IntCounter,
    pub edges_deleted: IntCounter,
    pub deltas_processed: IntCounter,
    pub deltas_failed: IntCounter,
}

/// Performance metrics
pub struct PerformanceMetrics {
    pub operation_duration: Histogram,
    pub memory_usage: Gauge,
    pub active_connections: IntGauge,
    pub queue_size: IntGauge,
    pub cpu_usage: Gauge,
}

/// Storage metrics
pub struct StorageMetrics {
    pub disk_usage: Gauge,
    pub disk_reads: IntCounter,
    pub disk_writes: IntCounter,
    pub cache_hits: IntCounter,
    pub cache_misses: IntCounter,
}

/// Network metrics
pub struct NetworkMetrics {
    pub bytes_sent: IntCounter,
    pub bytes_received: IntCounter,
    pub connections_accepted: IntCounter,
    pub connections_closed: IntCounter,
    pub messages_sent: IntCounter,
    pub messages_received: IntCounter,
}

/// Centralized metrics collection
pub struct Metrics {
    pub operations: OperationMetrics,
    pub performance: PerformanceMetrics,
    pub storage: StorageMetrics,
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

/// Timer for measuring operation duration
pub struct Timer {
    start: Instant,
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

/// Convenience macro for timing operations
#[macro_export]
macro_rules! time_operation {
    ($metric:expr, $body:expr) => {{
        let timer = $crate::metrics::Timer::start($metric.clone());
        let result = $body;
        timer.finish();
        result
    }};
}

/// Initialize the metrics registry
pub fn init_registry() {
    // Initialize global metrics to register them
    let _ = Metrics::global();
}

/// Get the Prometheus registry for serving metrics
pub fn registry() -> &'static Registry {
    &REGISTRY
}

/// Collect and return all metrics as a string
pub fn collect_metrics() -> String {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = registry().gather();
    encoder.encode_to_string(&metric_families).unwrap_or_default()
}

/// Update system metrics periodically
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
fn get_memory_usage() -> Result<usize> {
    // Simplified implementation - in production use a proper system monitoring crate
    Ok(0)
}

/// Get current CPU usage percentage
async fn get_cpu_usage() -> Result<f64> {
    // Simplified implementation - in production use a proper system monitoring crate
    Ok(0.0)
}

/// Get current disk usage in bytes
fn get_disk_usage() -> Result<usize> {
    // Simplified implementation - in production use a proper system monitoring crate
    Ok(0)
} 