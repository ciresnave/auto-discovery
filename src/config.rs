//! Configuration types for service discovery

use crate::types::{ProtocolType, ServiceType, DiscoveryFilter};
use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, time::Duration};

/// Configuration for the service discovery system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Service types to discover
    service_types: Vec<ServiceType>,
    /// Operation timeout
    timeout: Option<Duration>,
    /// Whether to verify discovered services
    verify_services: bool,
    /// Network interfaces to use
    interfaces: Option<HashSet<String>>,
    /// Maximum number of services to track
    max_services: usize,
    /// Maximum number of retries
    max_retries: u32,
    /// Cache duration
    cache_duration: Duration,
    /// Rate limit for discovery
    rate_limit: Option<Duration>,
    /// Whether metrics are enabled
    metrics_enabled: bool,
    /// Enabled protocols
    enabled_protocols: HashSet<ProtocolType>,
    /// Whether to allow cross-protocol discovery
    allow_cross_protocol: bool,
    /// Whether to enable IPv4 support
    enable_ipv4: bool,
    /// Whether to enable IPv6 support
    enable_ipv6: bool,
    /// Discovery filter
    filter: Option<DiscoveryFilter>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            service_types: Vec::new(),
            timeout: Some(Duration::from_secs(30)),
            verify_services: false,
            interfaces: None,
            max_services: 1000,
            max_retries: 3,
            cache_duration: Duration::from_secs(300),
            rate_limit: Some(Duration::from_secs(1)),
            metrics_enabled: false,
            enabled_protocols: [ProtocolType::Mdns].into_iter().collect(),
            allow_cross_protocol: false,
            enable_ipv4: true,
            enable_ipv6: false,
            filter: None,
        }
    }
}

impl DiscoveryConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set timeout for operations
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Get operation timeout
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    /// Set service types to discover
    pub fn with_service_type(mut self, service_type: ServiceType) -> Self {
        self.service_types.push(service_type);
        self
    }

    /// Get service types
    pub fn service_types(&self) -> &[ServiceType] {
        &self.service_types
    }

    /// Set service verification flag
    pub fn with_verify_services(mut self, verify: bool) -> Self {
        self.verify_services = verify;
        self
    }

    /// Check if service verification is enabled
    pub fn verify_services(&self) -> bool {
        self.verify_services
    }

    /// Set network interfaces
    pub fn with_interfaces(mut self, interfaces: HashSet<String>) -> Self {
        self.interfaces = Some(interfaces);
        self
    }

    /// Get network interfaces
    pub fn interfaces(&self) -> Option<&HashSet<String>> {
        self.interfaces.as_ref()
    }

    /// Set maximum number of services
    pub fn with_max_services(mut self, max: usize) -> Self {
        self.max_services = max;
        self
    }

    /// Get maximum number of services
    pub fn max_services(&self) -> usize {
        self.max_services
    }

    /// Set maximum number of retries
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Get maximum number of retries
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Enable IPv4 support
    pub fn with_ipv4(mut self, enable: bool) -> Self {
        self.enable_ipv4 = enable;
        self
    }

    /// Get IPv4 support status
    pub fn enable_ipv4(&self) -> bool {
        self.enable_ipv4
    }

    /// Enable IPv6 support
    pub fn with_ipv6(mut self, enable: bool) -> Self {
        self.enable_ipv6 = enable;
        self
    }

    /// Get IPv6 support status
    pub fn enable_ipv6(&self) -> bool {
        self.enable_ipv6
    }

    /// Enable a protocol
    pub fn with_protocol(mut self, protocol: ProtocolType) -> Self {
        self.enabled_protocols.insert(protocol);
        self
    }

    /// Check if a protocol is enabled
    pub fn is_protocol_enabled(&self, protocol: ProtocolType) -> bool {
        self.enabled_protocols.contains(&protocol)
    }

    /// Enable a protocol
    pub fn enable_protocol(&mut self, protocol: ProtocolType) {
        self.enabled_protocols.insert(protocol);
    }

    /// Disable a protocol
    pub fn disable_protocol(&mut self, protocol: ProtocolType) {
        self.enabled_protocols.remove(&protocol);
    }

    /// Set enabled protocols
    pub fn with_protocols(mut self, protocols: HashSet<ProtocolType>) -> Self {
        self.enabled_protocols = protocols;
        self
    }

    /// Get enabled protocols
    pub fn protocols(&self) -> &HashSet<ProtocolType> {
        &self.enabled_protocols
    }

    /// Enable cross-protocol discovery
    pub fn with_cross_protocol(mut self, enable: bool) -> Self {
        self.allow_cross_protocol = enable;
        self
    }

    /// Get cross-protocol discovery status
    pub fn allow_cross_protocol(&self) -> bool {
        self.allow_cross_protocol
    }

    /// Enable metrics
    pub fn with_metrics(mut self, enable: bool) -> Self {
        self.metrics_enabled = enable;
        self
    }

    /// Get metrics status
    pub fn metrics_enabled(&self) -> bool {
        self.metrics_enabled
    }

    /// Set rate limit
    pub fn with_rate_limit(mut self, limit: Duration) -> Self {
        self.rate_limit = Some(limit);
        self
    }

    /// Get rate limit
    pub fn rate_limit(&self) -> Option<Duration> {
        self.rate_limit
    }

    /// Set cache duration
    pub fn with_cache_duration(mut self, duration: Duration) -> Self {
        self.cache_duration = duration;
        self
    }

    /// Get cache duration
    pub fn cache_duration(&self) -> Duration {
        self.cache_duration
    }

    /// Get protocol timeout
    pub fn protocol_timeout(&self) -> Duration {
        self.timeout.unwrap_or(Duration::from_secs(30))
    }

    /// Check if a protocol is enabled (alias for is_protocol_enabled)
    pub fn has_protocol(&self, protocol: ProtocolType) -> bool {
        self.enabled_protocols.contains(&protocol)
    }

    /// Get the discovery filter
    pub fn filter(&self) -> Option<&DiscoveryFilter> {
        self.filter.as_ref()
    }

    /// Set discovery filter
    pub fn with_filter(mut self, filter: DiscoveryFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.timeout.is_some_and(|t| t.as_secs() == 0) {
            return Err(crate::error::DiscoveryError::configuration(
                "Timeout must be greater than 0",
            ));
        }

        if !self.enable_ipv4 && !self.enable_ipv6 {
            return Err(crate::error::DiscoveryError::configuration(
                "Either IPv4 or IPv6 must be enabled",
            ));
        }

        if self.enabled_protocols.is_empty() {
            return Err(crate::error::DiscoveryError::configuration(
                "At least one protocol must be enabled",
            ));
        }

        Ok(())
    }
}

/// Configuration for service registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationConfig {
    /// Time-to-live for the service record
    pub ttl: Duration,
    /// Whether to auto-refresh the service registration
    pub auto_refresh: bool,
    /// Refresh interval (only used if auto_refresh is true)
    pub refresh_interval: Duration,
    /// Network interfaces to register on
    pub interfaces: Vec<String>,
    /// Protocols to use for registration
    pub protocols: HashSet<ProtocolType>,
    /// Whether to enable IPv6 registration
    pub enable_ipv6: bool,
    /// Whether to enable IPv4 registration
    pub enable_ipv4: bool,
    /// Priority for the service (used in some protocols)
    pub priority: u16,
    /// Weight for the service (used in some protocols)
    pub weight: u16,
}

impl Default for RegistrationConfig {
    fn default() -> Self {
        let mut protocols = HashSet::new();
        protocols.insert(ProtocolType::Mdns);

        Self {
            ttl: Duration::from_secs(120),
            auto_refresh: true,
            refresh_interval: Duration::from_secs(60),
            interfaces: Vec::new(),
            protocols,
            enable_ipv6: true,
            enable_ipv4: true,
            priority: 0,
            weight: 0,
        }
    }
}

impl RegistrationConfig {
    /// Create a new registration configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the TTL for service records
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Enable or disable auto-refresh
    pub fn auto_refresh(mut self, auto_refresh: bool) -> Self {
        self.auto_refresh = auto_refresh;
        self
    }

    /// Set the refresh interval
    pub fn refresh_interval(mut self, interval: Duration) -> Self {
        self.refresh_interval = interval;
        self
    }

    /// Set network interfaces for registration
    pub fn interfaces<I, S>(mut self, interfaces: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.interfaces = interfaces.into_iter().map(|s| s.into()).collect();
        self
    }

    /// Set registration protocols
    pub fn protocols<I>(mut self, protocols: I) -> Self
    where
        I: IntoIterator<Item = ProtocolType>,
    {
        self.protocols = protocols.into_iter().collect();
        self
    }

    /// Set service priority
    pub fn priority(mut self, priority: u16) -> Self {
        self.priority = priority;
        self
    }

    /// Set service weight
    pub fn weight(mut self, weight: u16) -> Self {
        self.weight = weight;
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> crate::Result<()> {
        if self.ttl.is_zero() {
            return Err(crate::error::DiscoveryError::configuration(
                "TTL cannot be zero",
            ));
        }

        if self.auto_refresh && self.refresh_interval.is_zero() {
            return Err(crate::error::DiscoveryError::configuration(
                "Refresh interval cannot be zero when auto-refresh is enabled",
            ));
        }

        if self.auto_refresh && self.refresh_interval >= self.ttl {
            return Err(crate::error::DiscoveryError::configuration(
                "Refresh interval must be less than TTL",
            ));
        }

        if self.protocols.is_empty() {
            return Err(crate::error::DiscoveryError::configuration(
                "At least one protocol must be enabled",
            ));
        }

        if !self.enable_ipv4 && !self.enable_ipv6 {
            return Err(crate::error::DiscoveryError::configuration(
                "At least one IP version must be enabled",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_config_defaults() {
        let config = DiscoveryConfig::new();
        assert_eq!(config.service_types().len(), 0);
        assert!(config.timeout().is_some());
        assert!(!config.verify_services());
        assert!(config.is_protocol_enabled(ProtocolType::Mdns));
    }

    #[test]
    fn test_config_builder() -> Result<()> {
        let config = DiscoveryConfig::new()
            .with_service_type(ServiceType::new("_http._tcp")?);
        assert_eq!(config.service_types().len(), 1);
        Ok(())
    }

    #[test]
    fn test_config_validation() -> Result<()> {
        let config = DiscoveryConfig::new()
            .with_service_type(ServiceType::new("_http._tcp")?)
            .with_timeout(Duration::from_secs(10))
            .with_verify_services(true);

        assert!(config.validate().is_ok());

        // Test invalid timeout
        let invalid_config = DiscoveryConfig::new()
            .with_timeout(Duration::ZERO);
        assert!(invalid_config.validate().is_err());

        // Test invalid network config
        let invalid_config = DiscoveryConfig::new()
            .with_ipv4(false)
            .with_ipv6(false);
        assert!(invalid_config.validate().is_err());

        Ok(())
    }
}
