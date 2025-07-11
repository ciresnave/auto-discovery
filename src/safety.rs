//! Production safety features including rate limiting, timeouts, circuit breakers, and error recovery.

use std::{sync::Arc, time::Duration};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed, keyed::DefaultKeyedStateStore},
    RateLimiter,
    Quota,
};
use tokio_retry::{
    strategy::{ExponentialBackoff, jitter},
    Retry,
};
use metrics::{counter, gauge, histogram};
use parking_lot::RwLock;
use tracing::{debug, error, info, warn};
use crate::{error::Result, service::ServiceInfo};

/// Default rate limits (operations per second)
const DEFAULT_DISCOVERY_RATE: u32 = 10;
const DEFAULT_REGISTRATION_RATE: u32 = 5;
const DEFAULT_VERIFICATION_RATE: u32 = 20;

/// Default timeouts
const DEFAULT_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_VERIFICATION_TIMEOUT: Duration = Duration::from_secs(2);
const DEFAULT_REGISTRATION_TIMEOUT: Duration = Duration::from_secs(3);

/// Default retry settings
const MAX_RETRIES: u32 = 3;
const MIN_RETRY_DELAY: Duration = Duration::from_millis(100);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(5);

/// Circuit breaker settings
const CIRCUIT_BREAKER_THRESHOLD: u32 = 5;
const CIRCUIT_BREAKER_RESET_TIMEOUT: Duration = Duration::from_secs(30);

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for handling operation failures
pub struct CircuitBreaker {
    state: RwLock<CircuitState>,
    failures: RwLock<u32>,
    threshold: u32,
    reset_timeout: Duration,
    last_state_change: RwLock<std::time::Instant>,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failures: RwLock::new(0),
            threshold: CIRCUIT_BREAKER_THRESHOLD,
            reset_timeout: CIRCUIT_BREAKER_RESET_TIMEOUT,
            last_state_change: RwLock::new(std::time::Instant::now()),
        }
    }

    pub fn record_failure(&self) {
        let mut failures = self.failures.write();
        *failures += 1;

        if *failures >= self.threshold {
            let mut state = self.state.write();
            if *state == CircuitState::Closed {
                *state = CircuitState::Open;
                *self.last_state_change.write() = std::time::Instant::now();
                warn!("Circuit breaker opened after {} failures", failures);
                counter!("circuit_breaker_opens_total", 1);
            }
        }
    }

    pub fn record_success(&self) {
        let mut state = self.state.write();
        if *state == CircuitState::HalfOpen {
            *state = CircuitState::Closed;
            *self.failures.write() = 0;
            *self.last_state_change.write() = std::time::Instant::now();
            info!("Circuit breaker closed after successful operation");
            counter!("circuit_breaker_closes_total", 1);
        }
    }

    pub fn is_closed(&self) -> bool {
        let state = self.state.read();
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                let last_change = self.last_state_change.read();
                if last_change.elapsed() >= self.reset_timeout {
                    drop(last_change);
                    drop(state);
                    *self.state.write() = CircuitState::HalfOpen;
                    debug!("Circuit breaker entering half-open state");
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }
}

/// Rate limiter for service discovery operations with integrated circuit breakers
#[derive(Clone)]
pub struct SafetyManager {
    discovery_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    registration_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    verification_limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    discovery_breaker: Arc<CircuitBreaker>,
    registration_breaker: Arc<CircuitBreaker>,
    verification_breaker: Arc<CircuitBreaker>,
}

impl SafetyManager {
    /// Create a new safety manager with rate limiters and circuit breakers
    pub fn new() -> Self {
        Self {
            discovery_limiter: Arc::new(RateLimiter::direct(Quota::per_second(DEFAULT_DISCOVERY_RATE.into()))),
            registration_limiter: Arc::new(RateLimiter::direct(Quota::per_second(DEFAULT_REGISTRATION_RATE.into()))),
            verification_limiter: Arc::new(RateLimiter::direct(Quota::per_second(DEFAULT_VERIFICATION_RATE.into()))),
            discovery_breaker: Arc::new(CircuitBreaker::new()),
            registration_breaker: Arc::new(CircuitBreaker::new()),
            verification_breaker: Arc::new(CircuitBreaker::new()),
        }
    }

    /// Check if discovery operation is allowed
    pub fn check_discovery(&self) -> bool {
        if !self.discovery_breaker.is_closed() {
            counter!("safety_discovery_blocked_by_circuit_breaker", 1);
            return false;
        }

        match self.discovery_limiter.check() {
            Ok(_) => true,
            Err(_) => {
                counter!("safety_discovery_rate_limited", 1);
                false
            }
        }
    }

    /// Check if registration operation is allowed
    pub fn check_registration(&self) -> bool {
        if !self.registration_breaker.is_closed() {
            counter!("safety_registration_blocked_by_circuit_breaker", 1);
            return false;
        }

        match self.registration_limiter.check() {
            Ok(_) => true,
            Err(_) => {
                counter!("safety_registration_rate_limited", 1);
                false
            }
        }
    }

    /// Check if verification operation is allowed
    pub fn check_verification(&self) -> bool {
        if !self.verification_breaker.is_closed() {
            counter!("safety_verification_blocked_by_circuit_breaker", 1);
            return false;
        }

        match self.verification_limiter.check() {
            Ok(_) => true,
            Err(_) => {
                counter!("safety_verification_rate_limited", 1);
                false
            }
        }
    }

    /// Record operation success
    pub fn record_success(&self, operation: &str) {
        match operation {
            "discovery" => self.discovery_breaker.record_success(),
            "registration" => self.registration_breaker.record_success(),
            "verification" => self.verification_breaker.record_success(),
            _ => (),
        }
        counter!("safety_operation_success", 1, "operation" => operation.to_string());
    }

    /// Record operation failure
    pub fn record_failure(&self, operation: &str) {
        match operation {
            "discovery" => self.discovery_breaker.record_failure(),
            "registration" => self.registration_breaker.record_failure(),
            "verification" => self.verification_breaker.record_failure(),
            _ => (),
        }
        counter!("safety_operation_failure", 1, "operation" => operation.to_string());
    }

    /// Get retry strategy for an operation
    pub fn get_retry_strategy(&self) -> impl Iterator<Item = Duration> {
        ExponentialBackoff::from_millis(MIN_RETRY_DELAY.as_millis() as u64)
            .max_delay(MAX_RETRY_DELAY)
            .map(jitter)
            .take(MAX_RETRIES as usize)
    }

    /// Execute an operation with retries and safety checks
    pub async fn execute_with_safety<F, T>(&self, operation: &str, f: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>> + Send>>,
    {
        let retry_strategy = self.get_retry_strategy();
        let allowed = match operation {
            "discovery" => self.check_discovery(),
            "registration" => self.check_registration(),
            "verification" => self.check_verification(),
            _ => true,
        };

        if !allowed {
            return Err(crate::error::DiscoveryError::RateLimit(
                format!("Operation {} not allowed by safety checks", operation)
            ));
        }

        let start = std::time::Instant::now();
        let result = Retry::spawn(retry_strategy, f).await;
        histogram!("safety_operation_duration", start.elapsed(), "operation" => operation.to_string());

        match &result {
            Ok(_) => self.record_success(operation),
            Err(_) => self.record_failure(operation),
        }

        result
    }

    /// Get current circuit breaker states
    pub fn get_circuit_breaker_states(&self) -> Vec<(String, CircuitState)> {
        vec![
            ("discovery".to_string(), *self.discovery_breaker.state.read()),
            ("registration".to_string(), *self.registration_breaker.state.read()),
            ("verification".to_string(), *self.verification_breaker.state.read()),
        ]
    }
}

/// Retry strategy for fallible operations
pub struct RetryStrategy {
    max_retries: u32,
    strategy: ExponentialBackoff,
}

impl RetryStrategy {
    /// Create a new retry strategy with default settings
    pub fn new() -> Self {
        Self {
            max_retries: MAX_RETRIES,
            strategy: ExponentialBackoff::from_millis(MIN_RETRY_DELAY.as_millis() as u64)
                .max_delay(MAX_RETRY_DELAY)
                .map(jitter),
        }
    }

    /// Execute an operation with retries
    pub async fn retry<F, T, E>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<T, E>> + Send>>,
        E: std::fmt::Display,
    {
        let retry_strategy = self.strategy.clone();
        let max_retries = self.max_retries;

        Retry::spawn(retry_strategy.take(max_retries as usize), operation)
            .await
            .map_err(|e| crate::error::DiscoveryError::retry(e.to_string()))
    }
}

/// Service health monitoring
#[derive(Clone)]
pub struct HealthMonitor {
    services: Arc<RwLock<dashmap::DashMap<String, ServiceHealth>>>,
}

#[derive(Debug, Clone)]
struct ServiceHealth {
    last_seen: std::time::Instant,
    status: ServiceStatus,
    failure_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(dashmap::DashMap::new())),
        }
    }

    /// Update service health status
    pub fn update_service(&self, service: &ServiceInfo, healthy: bool) {
        let mut services = self.services.write();
        let entry = services.entry(service.id().to_string()).or_insert_with(|| ServiceHealth {
            last_seen: std::time::Instant::now(),
            status: ServiceStatus::Healthy,
            failure_count: 0,
        });

        if healthy {
            entry.failure_count = 0;
            entry.status = ServiceStatus::Healthy;
        } else {
            entry.failure_count += 1;
            if entry.failure_count > 3 {
                entry.status = ServiceStatus::Unhealthy;
            } else if entry.failure_count > 1 {
                entry.status = ServiceStatus::Degraded;
            }
        }

        entry.last_seen = std::time::Instant::now();

        // Update metrics
        gauge!("service_health", entry.status as i64, "service" => service.name().to_string());
        histogram!("service_failure_count", entry.failure_count as f64, "service" => service.name().to_string());
    }

    /// Get service health status
    pub fn get_service_status(&self, service_id: &str) -> Option<ServiceStatus> {
        self.services.read().get(service_id).map(|h| h.status)
    }

    /// Clean up stale service entries
    pub fn cleanup_stale(&self, max_age: Duration) {
        let mut services = self.services.write();
        services.retain(|_, health| health.last_seen.elapsed() < max_age);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = DiscoveryRateLimiter::new();
        
        // Test that rate limiting works
        for _ in 0..5 {
            limiter.check_discovery().await;
        }
    }

    #[tokio::test]
    async fn test_retry_strategy() {
        let retry = RetryStrategy::new();
        
        let success = retry.retry(|| Box::pin(async { Ok::<_, String>("success") })).await;
        assert!(success.is_ok());
        
        let mut attempts = 0;
        let failure = retry.retry(|| Box::pin(async move {
            attempts += 1;
            if attempts < 3 {
                Err("retry")?
            }
            Ok("success")
        })).await;
        assert!(failure.is_ok());
    }

    #[test]
    fn test_health_monitor() {
        let monitor = HealthMonitor::new();
        let service = ServiceInfo::new(
            "test_service",
            "_test._tcp",
            8080,
            None,
        );

        // Test health status updates
        monitor.update_service(&service, true);
        assert_eq!(monitor.get_service_status(&service.id().to_string()), Some(ServiceStatus::Healthy));

        // Test degradation
        monitor.update_service(&service, false);
        monitor.update_service(&service, false);
        assert_eq!(monitor.get_service_status(&service.id().to_string()), Some(ServiceStatus::Degraded));

        // Test cleanup
        monitor.cleanup_stale(Duration::from_secs(0));
        assert_eq!(monitor.get_service_status(&service.id().to_string()), None);
    }
}
