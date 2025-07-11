use crate::{
    config::DiscoveryConfig,
    error::{DiscoveryError, Result},
    protocols::ProtocolManager,
    security::tsig::TsigKeyManager,
    metrics,
};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use metrics::{counter, gauge, histogram};
use serde::Serialize;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tower::{ServiceBuilder, ServiceExt};
use tower_http::{
    trace::TraceLayer,
    compression::CompressionLayer,
    cors::CorsLayer,
};
use tracing::{debug, error, info, warn};

/// Health status of a component
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Health check configuration
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Check interval
    pub interval: Duration,
    /// Check timeout
    pub timeout: Duration,
    /// Failure threshold before marking as unhealthy
    pub failure_threshold: u32,
    /// Success threshold to restore health
    pub success_threshold: u32,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            failure_threshold: 3,
            success_threshold: 2,
        }
    }
}

/// Health check result for a component
#[derive(Debug, Clone, Serialize)]
pub struct HealthCheck {
    status: HealthStatus,
    message: Option<String>,
    last_check: chrono::DateTime<chrono::Utc>,
    consecutive_failures: u32,
    consecutive_successes: u32,
    last_failure: Option<chrono::DateTime<chrono::Utc>>,
    latency: Option<Duration>,
}

impl HealthCheck {
    fn new() -> Self {
        Self {
            status: HealthStatus::Healthy,
            message: None,
            last_check: chrono::Utc::now(),
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_failure: None,
            latency: None,
        }
    }

    fn record_success(&mut self, latency: Duration, config: &HealthConfig) {
        self.consecutive_successes += 1;
        self.consecutive_failures = 0;
        self.latency = Some(latency);
        self.last_check = chrono::Utc::now();

        if self.consecutive_successes >= config.success_threshold {
            if self.status != HealthStatus::Healthy {
                info!("Component restored to healthy status");
            }
            self.status = HealthStatus::Healthy;
        }

        histogram!("health_check_latency", latency);
        counter!("health_check_success", 1);
    }

    fn record_failure(&mut self, message: String, config: &HealthConfig) {
        self.consecutive_failures += 1;
        self.consecutive_successes = 0;
        self.message = Some(message);
        self.last_check = chrono::Utc::now();
        self.last_failure = Some(chrono::Utc::now());

        if self.consecutive_failures >= config.failure_threshold {
            if self.status == HealthStatus::Healthy {
                warn!("Component marked as unhealthy: {}", self.message.as_ref().unwrap());
            }
            self.status = HealthStatus::Unhealthy;
        } else if self.status == HealthStatus::Healthy {
            self.status = HealthStatus::Degraded;
        }

        counter!("health_check_failure", 1);
    }
}

/// Health check results for all components
#[derive(Debug, Clone, Serialize)]
pub struct HealthReport {
    status: HealthStatus,
    components: HashMap<String, HealthCheck>,
    uptime: Duration,
    version: String,
    start_time: chrono::DateTime<chrono::Utc>,
    memory_usage: Option<u64>,
    thread_count: Option<u64>,
}

impl HealthReport {
    fn new() -> Self {
        Self {
            status: HealthStatus::Healthy,
            components: HashMap::new(),
            uptime: Duration::from_secs(0),
            version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: chrono::Utc::now(),
            memory_usage: None,
            thread_count: None,
        }
    }

    fn update_system_metrics(&mut self) {
        if let Ok(mem_info) = sys_info::mem_info() {
            self.memory_usage = Some(mem_info.total - mem_info.avail);
            gauge!("system_memory_usage", self.memory_usage.unwrap() as f64);
        }

        if let Ok(proc_stat) = sys_info::proc_stat() {
            self.thread_count = Some(proc_stat.procs as u64);
            gauge!("system_thread_count", self.thread_count.unwrap() as f64);
        }
    }

    fn calculate_overall_status(&mut self) {
        let unhealthy = self.components.values().any(|c| c.status == HealthStatus::Unhealthy);
        let degraded = self.components.values().any(|c| c.status == HealthStatus::Degraded);

        self.status = if unhealthy {
            HealthStatus::Unhealthy
        } else if degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        };

        gauge!("health_status", match self.status {
            HealthStatus::Healthy => 2.0,
            HealthStatus::Degraded => 1.0,
            HealthStatus::Unhealthy => 0.0,
        });
    }
}

/// Health monitor service
pub struct HealthMonitor {
    config: HealthConfig,
    report: Arc<RwLock<HealthReport>>,
    protocol_manager: Arc<RwLock<ProtocolManager>>,
    tsig_manager: Arc<TsigKeyManager>,
}

impl HealthMonitor {
    pub fn new(
        config: HealthConfig,
        protocol_manager: Arc<RwLock<ProtocolManager>>,
        tsig_manager: Arc<TsigKeyManager>,
    ) -> Self {
        Self {
            config,
            report: Arc::new(RwLock::new(HealthReport::new())),
            protocol_manager,
            tsig_manager,
        }
    }

    /// Start the health monitoring service
    pub async fn start(self: Arc<Self>, addr: SocketAddr) -> Result<()> {
        // Start periodic health checks
        self.start_health_checks().await;

        // Start HTTP health check endpoint
        self.start_http_server(addr).await?;

        info!("Health monitoring service started on {}", addr);
        Ok(())
    }

    async fn start_health_checks(self: Arc<Self>) {
        let config = self.config.clone();
        let report = self.report.clone();
        let protocol_manager = self.protocol_manager.clone();
        let tsig_manager = self.tsig_manager.clone();

        tokio::spawn(async move {
            loop {
                let start = Instant::now();
                let mut report = report.write().await;

                // Update system metrics
                report.update_system_metrics();
                report.uptime = start.duration_since(report.start_time.into());

                // Check protocol manager
                Self::check_protocol_manager(&mut report, &protocol_manager, &config).await;

                // Check security components
                Self::check_security_components(&mut report, &tsig_manager, &config).await;

                // Calculate overall status
                report.calculate_overall_status();

                histogram!("health_check_duration", start.elapsed());
                tokio::time::sleep(config.interval).await;
            }
        });
    }

    async fn check_protocol_manager(
        report: &mut HealthReport,
        protocol_manager: &Arc<RwLock<ProtocolManager>>,
        config: &HealthConfig,
    ) {
        let start = Instant::now();
        let manager = protocol_manager.read().await;

        match manager.get_protocol_status().await {
            Ok(status) => {
                let check = report.components.entry("protocol_manager".to_string())
                    .or_insert_with(HealthCheck::new);
                check.record_success(start.elapsed(), config);
                
                // Record protocol-specific metrics
                for (protocol, is_healthy) in status {
                    let metric_name = format!("protocol_{}_healthy", protocol);
                    gauge!(&metric_name, if is_healthy { 1.0 } else { 0.0 });
                }
            }
            Err(e) => {
                let check = report.components.entry("protocol_manager".to_string())
                    .or_insert_with(HealthCheck::new);
                check.record_failure(e.to_string(), config);
            }
        }
    }

    async fn check_security_components(
        report: &mut HealthReport,
        tsig_manager: &Arc<TsigKeyManager>,
        config: &HealthConfig,
    ) {
        let start = Instant::now();
        
        match tsig_manager.verify_keys().await {
            Ok(_) => {
                let check = report.components.entry("security_manager".to_string())
                    .or_insert_with(HealthCheck::new);
                check.record_success(start.elapsed(), config);
            }
            Err(e) => {
                let check = report.components.entry("security_manager".to_string())
                    .or_insert_with(HealthCheck::new);
                check.record_failure(e.to_string(), config);
            }
        }
    }

    async fn start_http_server(self: Arc<Self>, addr: SocketAddr) -> Result<()> {
        let report = self.report.clone();

        let make_svc = make_service_fn(move |_conn| {
            let report = report.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |_req: Request<Body>| {
                    let report = report.clone();
                    async move {
                        let report = report.read().await;
                        let json = serde_json::to_string(&*report)
                            .map_err(|e| DiscoveryError::Internal(e.to_string()))?;
                        
                        Ok::<_, DiscoveryError>(Response::builder()
                            .status(if report.status == HealthStatus::Healthy { 200 } else { 503 })
                            .header("Content-Type", "application/json")
                            .body(Body::from(json))
                            .unwrap())
                    }
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);
        tokio::spawn(server);

        Ok(())
    }

    /// Get the current health report
    pub async fn get_report(&self) -> HealthReport {
        self.report.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Client;

    #[tokio::test]
    async fn test_health_check_server() {
        let config = HealthConfig::builder()
            .port(0) // Random port
            .build();

        let protocol_manager = Arc::new(ProtocolManager::new(&DiscoveryConfig::default()).unwrap());
        let health_manager = Arc::new(HealthManager::new(
            config,
            protocol_manager,
            None,
        ));

        // Start server
        let server_handle = tokio::spawn(health_manager.clone().start());

        // Wait for server to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Make request
        let client = Client::new();
        let uri = format!("http://127.0.0.1:{}/health", health_manager.config.port);
        let response = client.get(uri.parse().unwrap()).await.unwrap();

        assert_eq!(response.status(), 200);

        // Parse response
        let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let report: HealthReport = serde_json::from_slice(&body_bytes).unwrap();

        assert!(matches!(report.status, HealthStatus::Healthy));
        assert!(!report.components.is_empty());

        // Cleanup
        server_handle.abort();
    }

    #[tokio::test]
    async fn test_component_health_tracking() {
        let config = HealthConfig::builder()
            .check_interval(Duration::from_millis(100))
            .build();

        let protocol_manager = Arc::new(ProtocolManager::new(&DiscoveryConfig::default()).unwrap());
        let health_manager = Arc::new(HealthManager::new(
            config,
            protocol_manager.clone(),
            None,
        ));

        // Start health checks
        health_manager.clone().start_health_checks().await;

        // Wait for checks to run
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify checks were performed
        let checks = health_manager.health_checks.read().await;
        assert!(!checks.is_empty());

        // Verify timestamps are recent
        let now = chrono::Utc::now();
        for check in checks.values() {
            assert!(now.signed_duration_since(check.last_check).num_seconds() < 2);
        }
    }
}
