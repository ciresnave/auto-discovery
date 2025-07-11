use std::sync::Arc;
use parking_lot::RwLock;
use tower::load::PeakEwma;
use tower::discover::Change;
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use crate::service::ServiceInfo;
use crate::error::Result;

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    LeastLoaded,
    Random,
}

/// Load balancer configuration
#[derive(Debug, Clone)]
pub struct LoadBalancerConfig {
    pub strategy: LoadBalancingStrategy,
    pub decay_time: std::time::Duration,
    pub rtt_threshold: std::time::Duration,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            strategy: LoadBalancingStrategy::LeastLoaded,
            decay_time: std::time::Duration::from_secs(10),
            rtt_threshold: std::time::Duration::from_millis(100),
        }
    }
}

/// Service load statistics
#[derive(Debug, Clone)]
pub struct ServiceLoad {
    pub service: ServiceInfo,
    pub current_load: f64,
    pub response_time: std::time::Duration,
    pub success_rate: f64,
}

/// Load balancer for service discovery
pub struct LoadBalancer {
    config: LoadBalancerConfig,
    services: Arc<RwLock<Vec<ServiceLoad>>>,
    load_metrics: Arc<RwLock<dashmap::DashMap<String, PeakEwma>>>,
    changes_tx: mpsc::Sender<Change<String, ServiceLoad>>,
    changes_rx: mpsc::Receiver<Change<String, ServiceLoad>>,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(config: LoadBalancerConfig) -> Self {
        let (changes_tx, changes_rx) = mpsc::channel(100);
        
        Self {
            config,
            services: Arc::new(RwLock::new(Vec::new())),
            load_metrics: Arc::new(RwLock::new(dashmap::DashMap::new())),
            changes_tx,
            changes_rx,
        }
    }

    /// Add or update a service
    pub async fn update_service(&self, service: ServiceInfo, load: f64) -> Result<()> {
        let service_load = ServiceLoad {
            service: service.clone(),
            current_load: load,
            response_time: std::time::Duration::default(),
            success_rate: 1.0,
        };

        // Update load metrics
        self.load_metrics.write().entry(service.id().to_string())
            .or_insert_with(|| PeakEwma::new(
                self.config.decay_time,
                self.config.rtt_threshold,
            ));

        // Send change notification
        self.changes_tx.send(Change::Insert(service.id().to_string(), service_load)).await
            .map_err(|e| crate::error::DiscoveryError::other(format!("Failed to send change: {}", e)))?;

        Ok(())
    }

    /// Remove a service
    pub async fn remove_service(&self, service_id: &str) -> Result<()> {
        self.load_metrics.write().remove(service_id);
        self.changes_tx.send(Change::Remove(service_id.to_string())).await
            .map_err(|e| crate::error::DiscoveryError::other(format!("Failed to send removal: {}", e)))?;
        Ok(())
    }

    /// Select the best service based on the configured strategy
    pub fn select_service(&self) -> Option<ServiceInfo> {
        let services = self.services.read();
        if services.is_empty() {
            return None;
        }

        match self.config.strategy {
            LoadBalancingStrategy::RoundRobin => {
                // Simple round-robin selection
                let next_index = rand::random::<usize>() % services.len();
                Some(services[next_index].service.clone())
            }
            LoadBalancingStrategy::LeastLoaded => {
                // Select service with lowest load
                services.iter()
                    .min_by(|a, b| a.current_load.partial_cmp(&b.current_load).unwrap())
                    .map(|s| s.service.clone())
            }
            LoadBalancingStrategy::Random => {
                // Random selection weighted by inverse load
                let total_inverse_load: f64 = services.iter()
                    .map(|s| 1.0 / (s.current_load + 1.0))
                    .sum();

                let mut random = rand::random::<f64>() * total_inverse_load;
                for service in services.iter() {
                    let inverse_load = 1.0 / (service.current_load + 1.0);
                    if random <= inverse_load {
                        return Some(service.service.clone());
                    }
                    random -= inverse_load;
                }
                None
            }
        }
    }

    /// Update service metrics based on request result
    pub fn record_request(&self, service_id: &str, duration: std::time::Duration, success: bool) {
        if let Some(mut metric) = self.load_metrics.write().get_mut(service_id) {
            metric.record_rtt(duration);
        }

        if let Some(services) = self.services.try_write() {
            if let Some(service) = services.iter_mut().find(|s| s.service.id() == service_id) {
                service.response_time = duration;
                if !success {
                    service.success_rate *= 0.95; // Decay success rate on failure
                }
            }
        }

        // Update prometheus metrics
        metrics::histogram!("service_response_time", duration.as_secs_f64(),
            "service_id" => service_id.to_string()
        );
        metrics::counter!("service_request_total",
            "service_id" => service_id.to_string(),
            "success" => success.to_string()
        ).increment(1);
    }
}

impl Stream for LoadBalancer {
    type Item = Change<String, ServiceLoad>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.changes_rx.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_load_balancer() {
        let config = LoadBalancerConfig::default();
        let balancer = LoadBalancer::new(config);

        // Add test services
        let service1 = ServiceInfo::new("service1", "_test._tcp", 8080, None);
        let service2 = ServiceInfo::new("service2", "_test._tcp", 8081, None);

        balancer.update_service(service1.clone(), 0.5).await.unwrap();
        balancer.update_service(service2.clone(), 1.0).await.unwrap();

        // Test service selection
        let selected = balancer.select_service().unwrap();
        assert!(selected.id() == service1.id() || selected.id() == service2.id());

        // Test metric recording
        balancer.record_request(&service1.id(), std::time::Duration::from_millis(50), true);
        balancer.record_request(&service2.id(), std::time::Duration::from_millis(100), false);

        // Test service removal
        balancer.remove_service(&service1.id()).await.unwrap();
        assert_eq!(balancer.select_service().unwrap().id(), service2.id());
    }
}
