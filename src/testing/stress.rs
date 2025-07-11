use crate::{
    config::DiscoveryConfig,
    error::Result,
    protocols::{DiscoveryProtocol, dns_sd::DnsSdProtocol},
    service::ServiceInfo,
    types::{ProtocolType, ServiceType},
};
use dashmap::DashMap;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{Barrier, Semaphore},
    task::JoinSet,
    time::sleep,
};
use tracing::{debug, info, warn};

/// Stress test configuration
pub struct StressTestConfig {
    /// Number of concurrent services to register
    pub service_count: usize,
    /// Number of concurrent discovery operations
    pub discovery_concurrency: usize,
    /// Duration to run the test
    pub test_duration: Duration,
    /// Rate limit for operations (per second)
    pub rate_limit: Option<u32>,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            service_count: 100,
            discovery_concurrency: 10,
            test_duration: Duration::from_secs(60),
            rate_limit: Some(1000),
        }
    }
}

/// Stress test results
#[derive(Debug)]
pub struct StressTestResults {
    /// Total number of successful registrations
    pub successful_registrations: usize,
    /// Total number of failed registrations
    pub failed_registrations: usize,
    /// Total number of successful discoveries
    pub successful_discoveries: usize,
    /// Total number of failed discoveries
    pub failed_discoveries: usize,
    /// Average discovery latency
    pub avg_discovery_latency: Duration,
    /// Maximum discovery latency
    pub max_discovery_latency: Duration,
    /// Test duration
    pub test_duration: Duration,
}

/// Run a stress test on the service discovery system
pub async fn run_stress_test(config: StressTestConfig) -> Result<StressTestResults> {
    let start_time = Instant::now();
    let discovery_config = DiscoveryConfig::default();
    let protocol = Arc::new(DnsSdProtocol::new(discovery_config).await?);

    // Create rate limiter if configured
    let rate_limiter = config.rate_limit.map(|limit| {
        Arc::new(governor::RateLimiter::direct(
            governor::Quota::per_second(limit.into())
        ))
    });

    // Metrics tracking
    let metrics = Arc::new(DashMap::new());
    let barrier = Arc::new(Barrier::new(2)); // For registration and discovery tasks

    // Start registration task
    let registration_metrics = metrics.clone();
    let reg_barrier = barrier.clone();
    let reg_protocol = protocol.clone();
    let registration_handle = tokio::spawn(async move {
        run_registration_test(
            reg_protocol,
            config.service_count,
            rate_limiter.clone(),
            registration_metrics,
        ).await;
        reg_barrier.wait().await;
    });

    // Start discovery task
    let discovery_metrics = metrics.clone();
    let disc_barrier = barrier.clone();
    let disc_protocol = protocol.clone();
    let discovery_handle = tokio::spawn(async move {
        run_discovery_test(
            disc_protocol,
            config.discovery_concurrency,
            config.test_duration,
            rate_limiter,
            discovery_metrics,
        ).await;
        disc_barrier.wait().await;
    });

    // Wait for test completion
    let _ = tokio::join!(registration_handle, discovery_handle);

    // Calculate results
    let mut results = StressTestResults {
        successful_registrations: 0,
        failed_registrations: 0,
        successful_discoveries: 0,
        failed_discoveries: 0,
        avg_discovery_latency: Duration::default(),
        max_discovery_latency: Duration::default(),
        test_duration: start_time.elapsed(),
    };

    // Process metrics
    let mut total_latency = Duration::default();
    let mut latency_count = 0;

    for entry in metrics.iter() {
        match *entry.key() {
            "successful_registrations" => results.successful_registrations = *entry.value(),
            "failed_registrations" => results.failed_registrations = *entry.value(),
            "successful_discoveries" => results.successful_discoveries = *entry.value(),
            "failed_discoveries" => results.failed_discoveries = *entry.value(),
            "discovery_latency" => {
                let latency: Duration = *entry.value();
                total_latency += latency;
                latency_count += 1;
                if latency > results.max_discovery_latency {
                    results.max_discovery_latency = latency;
                }
            }
            _ => {}
        }
    }

    if latency_count > 0 {
        results.avg_discovery_latency = total_latency / latency_count as u32;
    }

    Ok(results)
}

async fn run_registration_test(
    protocol: Arc<DnsSdProtocol>,
    service_count: usize,
    rate_limiter: Option<Arc<governor::RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>>,
    metrics: Arc<DashMap<&'static str, usize>>,
) {
    let mut tasks = JoinSet::new();
    let semaphore = Arc::new(Semaphore::new(20)); // Limit concurrent registrations

    for i in 0..service_count {
        let protocol = protocol.clone();
        let rate_limiter = rate_limiter.clone();
        let metrics = metrics.clone();
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        tasks.spawn(async move {
            if let Some(limiter) = rate_limiter.as_ref() {
                limiter.until_ready().await;
            }

            let service = ServiceInfo {
                id: format!("stress-test-{}", i),
                name: format!("Stress Test Service {}", i),
                service_type: ServiceType {
                    name: "_test".to_string(),
                    protocol: "_tcp".to_string(),
                },
                address: SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                    8000 + i as u16,
                ),
                txt_records: Default::default(),
                protocol: ProtocolType::DnsSd,
                ttl: Duration::from_secs(3600),
            };

            match protocol.register_service(service).await {
                Ok(_) => {
                    metrics.entry("successful_registrations")
                        .and_modify(|e| *e += 1)
                        .or_insert(1);
                }
                Err(_) => {
                    metrics.entry("failed_registrations")
                        .and_modify(|e| *e += 1)
                        .or_insert(1);
                }
            }

            drop(permit);
        });
    }

    while tasks.join_next().await.is_some() {}
}

async fn run_discovery_test(
    protocol: Arc<DnsSdProtocol>,
    concurrency: usize,
    duration: Duration,
    rate_limiter: Option<Arc<governor::RateLimiter<governor::state::NotKeyed, governor::state::InMemoryState, governor::clock::DefaultClock>>>,
    metrics: Arc<DashMap<&'static str, usize>>,
) {
    let end_time = Instant::now() + duration;
    let mut tasks = JoinSet::new();
    let semaphore = Arc::new(Semaphore::new(concurrency));

    while Instant::now() < end_time {
        let protocol = protocol.clone();
        let rate_limiter = rate_limiter.clone();
        let metrics = metrics.clone();
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        tasks.spawn(async move {
            if let Some(limiter) = rate_limiter.as_ref() {
                limiter.until_ready().await;
            }

            let start = Instant::now();
            match protocol.discover_services(
                vec![ServiceType {
                    name: "_test".to_string(),
                    protocol: "_tcp".to_string(),
                }],
                Duration::from_secs(5),
            ).await {
                Ok(services) => {
                    let latency = start.elapsed();
                    metrics.insert("discovery_latency", latency);
                    metrics.entry("successful_discoveries")
                        .and_modify(|e| *e += 1)
                        .or_insert(1);
                    debug!("Discovered {} services", services.len());
                }
                Err(_) => {
                    metrics.entry("failed_discoveries")
                        .and_modify(|e| *e += 1)
                        .or_insert(1);
                }
            }

            drop(permit);
        });

        sleep(Duration::from_millis(100)).await;
    }

    while tasks.join_next().await.is_some() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stress_test() {
        let config = StressTestConfig {
            service_count: 10,
            discovery_concurrency: 2,
            test_duration: Duration::from_secs(5),
            rate_limit: Some(100),
        };

        let results = run_stress_test(config).await.unwrap();
        
        assert!(results.successful_registrations > 0);
        assert!(results.successful_discoveries > 0);
        assert!(results.avg_discovery_latency > Duration::from_millis(0));
    }
}
