//! Metrics collection and reporting for service discovery operations

use metrics::{
    counter, gauge, histogram,
    register_counter, register_gauge, register_histogram,
};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use once_cell::sync::OnceCell;
use std::{net::SocketAddr, time::Duration};
use tokio::task;
use tracing::{debug, error, info};

/// Global metrics handle
static METRICS: OnceCell<PrometheusHandle> = OnceCell::new();

/// Metric labels and descriptions
const METRIC_PREFIX: &str = "auto_discovery";
const METRIC_LABELS: &[&str] = &["protocol", "service_type", "operation"];

/// Register metrics for collection
pub fn register_metrics() {
    // Protocol metrics
    register_counter!("protocol_operations_total", "Total number of protocol operations");
    register_counter!("protocol_errors_total", "Total number of protocol errors");
    register_histogram!("protocol_operation_duration", "Duration of protocol operations");
    register_gauge!("protocol_active_services", "Number of active services per protocol");

    // Discovery metrics
    register_counter!("discovery_attempts_total", "Total number of service discovery attempts");
    register_counter!("services_discovered_total", "Total number of services discovered");
    register_histogram!("discovery_time", "Time taken for service discovery");
    register_gauge!("active_discoveries", "Number of active discovery operations");

    // Registration metrics
    register_counter!("service_registrations_total", "Total number of service registrations");
    register_counter!("service_unregistrations_total", "Total number of service unregistrations");
    register_gauge!("registered_services", "Number of currently registered services");

    // Rate limiting metrics
    register_counter!("rate_limit_exceeded_total", "Total number of rate limit exceeded events");
    register_gauge!("rate_limit_remaining", "Remaining rate limit quota");

    // Network metrics
    register_counter!("network_errors_total", "Total number of network errors");
    register_histogram!("network_latency", "Network operation latency");
    register_gauge!("active_connections", "Number of active network connections");

    // Cache metrics
    register_gauge!("cache_size", "Current size of the service cache");
    register_counter!("cache_hits_total", "Total number of cache hits");
    register_counter!("cache_misses_total", "Total number of cache misses");

    // Health check metrics
    register_counter!("health_checks_total", "Total number of health checks performed");
    register_counter!("health_check_failures_total", "Total number of failed health checks");
    register_gauge!("healthy_services", "Number of currently healthy services");
}

/// Initialize metrics collection with Prometheus export
pub async fn init_metrics(addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize metrics registry
    register_metrics();

    // Configure Prometheus builder
    let builder = PrometheusBuilder::new()
        .with_http_listener(addr)
        .add_global_label("service", "auto-discovery")
        .set_buckets(&[
            0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ])?;

    // Install global recorder
    let handle = builder.install_recorder()?;
    METRICS.set(handle).expect("Failed to set metrics handle");

    info!("Metrics server started on {}", addr);
    Ok(())
}

/// Record protocol operation timing
pub fn record_operation_duration(protocol: &str, operation: &str, duration: Duration) {
    histogram!(
        "protocol_operation_duration",
        duration,
        "protocol" => protocol.to_string(),
        "operation" => operation.to_string()
    );
}

/// Record service discovery
pub fn record_service_discovery(protocol: &str, service_type: &str, count: u64) {
    counter!(
        "services_discovered_total",
        count,
        "protocol" => protocol.to_string(),
        "service_type" => service_type.to_string()
    );
    gauge!(
        "active_discoveries",
        -1.0,
        "protocol" => protocol.to_string()
    );
}

/// Record service registration
pub fn record_service_registration(protocol: &str, service_type: &str) {
    counter!(
        "service_registrations_total",
        1,
        "protocol" => protocol.to_string(),
        "service_type" => service_type.to_string()
    );
    gauge!(
        "registered_services",
        1.0,
        "protocol" => protocol.to_string()
    );
}

/// Record service unregistration
pub fn record_service_unregistration(protocol: &str, service_type: &str) {
    counter!(
        "service_unregistrations_total",
        1,
        "protocol" => protocol.to_string(),
        "service_type" => service_type.to_string()
    );
    gauge!(
        "registered_services",
        -1.0,
        "protocol" => protocol.to_string()
    );
}

/// Record rate limit event
pub fn record_rate_limit(protocol: &str, operation: &str, remaining: f64) {
    counter!(
        "rate_limit_exceeded_total",
        1,
        "protocol" => protocol.to_string(),
        "operation" => operation.to_string()
    );
    gauge!(
        "rate_limit_remaining",
        remaining,
        "protocol" => protocol.to_string()
    );
}

/// Record network error
pub fn record_network_error(protocol: &str, error_type: &str) {
    counter!(
        "network_errors_total",
        1,
        "protocol" => protocol.to_string(),
        "error_type" => error_type.to_string()
    );
}

/// Record cache operation
pub fn record_cache_operation(operation: &str, hit: bool) {
    if hit {
        counter!("cache_hits_total", 1, "operation" => operation.to_string());
    } else {
        counter!("cache_misses_total", 1, "operation" => operation.to_string());
    }
}

/// Record health check result
pub fn record_health_check(protocol: &str, service_type: &str, healthy: bool) {
    counter!(
        "health_checks_total",
        1,
        "protocol" => protocol.to_string(),
        "service_type" => service_type.to_string()
    );
    
    if !healthy {
        counter!(
            "health_check_failures_total",
            1,
            "protocol" => protocol.to_string(),
            "service_type" => service_type.to_string()
        );
    }
    
    gauge!(
        "healthy_services",
        if healthy { 1.0 } else { -1.0 },
        "protocol" => protocol.to_string()
    );
}

/// Metric recording helper for async operations
pub async fn record_async_operation<F, T>(
    protocol: &str,
    operation: &str,
    f: F,
) -> Result<T, Box<dyn std::error::Error>>
where
    F: std::future::Future<Output = Result<T, Box<dyn std::error::Error>>>,
{
    let start = std::time::Instant::now();
    counter!(
        "protocol_operations_total",
        1,
        "protocol" => protocol.to_string(),
        "operation" => operation.to_string()
    );

    match f.await {
        Ok(result) => {
            record_operation_duration(protocol, operation, start.elapsed());
            Ok(result)
        }
        Err(e) => {
            counter!(
                "protocol_errors_total",
                1,
                "protocol" => protocol.to_string(),
                "operation" => operation.to_string()
            );
            Err(e)
        }
    }
}
