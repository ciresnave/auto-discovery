//! Service Registry for managing discovered and registered services
//!
//! This module provides a centralized registry for managing service discovery state
//! across different protocols. It handles both locally registered services and
//! services discovered from the network.

use crate::{
    error::{DiscoveryError, Result},
    service::ServiceInfo,
    types::{ServiceType, ProtocolType},
};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Entry in the service registry with metadata
#[derive(Debug, Clone)]
pub struct ServiceEntry {
    /// The service information
    pub service: ServiceInfo,
    /// When the service was registered/discovered
    pub timestamp: Instant,
    /// Whether this is a locally registered service
    pub is_local: bool,
    /// Time-to-live for the service entry
    pub ttl: Option<Duration>,
    /// The protocol that discovered/registered this service
    pub protocol: ProtocolType,
}

impl ServiceEntry {
    /// Create a new service entry for a locally registered service
    pub fn new_local(service: ServiceInfo, protocol: ProtocolType) -> Self {
        Self {
            service,
            timestamp: Instant::now(),
            is_local: true,
            ttl: None, // Local services don't expire
            protocol,
        }
    }

    /// Create a new service entry for a discovered service
    pub fn new_discovered(service: ServiceInfo, protocol: ProtocolType, ttl: Option<Duration>) -> Self {
        Self {
            service,
            timestamp: Instant::now(),
            is_local: false,
            ttl,
            protocol,
        }
    }

    /// Check if this service entry has expired
    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl {
            self.timestamp.elapsed() > ttl
        } else {
            false
        }
    }

    /// Get the service ID for indexing
    pub fn service_id(&self) -> String {
        format!("{}:{}:{}", self.service.name(), self.service.service_type(), self.service.port())
    }
}

/// Filter for querying services from the registry
#[derive(Debug, Clone, Default)]
pub struct ServiceFilter {
    /// Filter by service types
    pub service_types: Option<Vec<ServiceType>>,
    /// Filter by protocols
    pub protocols: Option<Vec<ProtocolType>>,
    /// Filter by service name (contains)
    pub name_contains: Option<String>,
    /// Include only local services
    pub local_only: bool,
    /// Include only discovered services
    pub discovered_only: bool,
    /// Maximum age of services to include
    pub max_age: Option<Duration>,
}



impl ServiceFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by service types
    pub fn with_service_types(mut self, types: Vec<ServiceType>) -> Self {
        self.service_types = Some(types);
        self
    }

    /// Filter by protocols
    pub fn with_protocols(mut self, protocols: Vec<ProtocolType>) -> Self {
        self.protocols = Some(protocols);
        self
    }

    /// Filter by service name
    pub fn with_name_contains(mut self, name: String) -> Self {
        self.name_contains = Some(name);
        self
    }

    /// Include only local services
    pub fn local_only(mut self) -> Self {
        self.local_only = true;
        self
    }

    /// Include only discovered services
    pub fn discovered_only(mut self) -> Self {
        self.discovered_only = true;
        self
    }

    /// Set maximum age of services
    pub fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = Some(max_age);
        self
    }

    /// Check if a service entry matches this filter
    pub fn matches(&self, entry: &ServiceEntry) -> bool {
        // Check if expired
        if entry.is_expired() {
            return false;
        }

        // Check max age
        if let Some(max_age) = self.max_age {
            if entry.timestamp.elapsed() > max_age {
                return false;
            }
        }

        // Check local/discovered filter
        if self.local_only && !entry.is_local {
            return false;
        }
        if self.discovered_only && entry.is_local {
            return false;
        }

        // Check service types
        if let Some(ref types) = self.service_types {
            if !types.iter().any(|t| t.to_string() == entry.service.service_type().to_string()) {
                return false;
            }
        }

        // Check protocols
        if let Some(ref protocols) = self.protocols {
            if !protocols.contains(&entry.protocol) {
                return false;
            }
        }

        // Check name contains
        if let Some(ref name) = self.name_contains {
            if !entry.service.name().contains(name) {
                return false;
            }
        }

        true
    }
}

/// Centralized service registry for managing discovered and registered services
pub struct ServiceRegistry {
    /// All services indexed by service ID
    services: Arc<RwLock<HashMap<String, ServiceEntry>>>,
    /// Default TTL for discovered services
    default_ttl: Duration,
    /// Maximum number of services to store
    max_services: usize,
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Duration::from_secs(300), // 5 minutes
            max_services: 1000,
        }
    }

    /// Create a new service registry with custom settings
    pub fn with_settings(default_ttl: Duration, max_services: usize) -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
            max_services,
        }
    }

    /// Register a local service
    pub async fn register_local_service(&self, service: ServiceInfo, protocol: ProtocolType) -> Result<()> {
        let entry = ServiceEntry::new_local(service, protocol);
        let service_id = entry.service_id();
        
        let mut services = self.services.write().await;
        services.insert(service_id.clone(), entry);
        
        info!("Registered local service: {}", service_id);
        Ok(())
    }

    /// Unregister a local service
    pub async fn unregister_local_service(&self, service_id: &str) -> Result<()> {
        let mut services = self.services.write().await;
        if services.remove(service_id).is_some() {
            info!("Unregistered local service: {}", service_id);
            Ok(())
        } else {
            warn!("Attempted to unregister unknown service: {}", service_id);
            Err(DiscoveryError::service_not_found(service_id))
        }
    }

    /// Add a discovered service
    pub async fn add_discovered_service(&self, service: ServiceInfo, protocol: ProtocolType, ttl: Option<Duration>) -> Result<()> {
        let ttl = ttl.unwrap_or(self.default_ttl);
        let entry = ServiceEntry::new_discovered(service, protocol, Some(ttl));
        let service_id = entry.service_id();
        
        let mut services = self.services.write().await;
        
        // Check if we're at capacity
        if services.len() >= self.max_services {
            // Remove oldest expired service
            if let Some(oldest_expired) = self.find_oldest_expired(&services) {
                services.remove(&oldest_expired);
            } else {
                warn!("Service registry at capacity, cannot add new service");
                return Err(DiscoveryError::configuration("Service registry at capacity"));
            }
        }
        
        services.insert(service_id.clone(), entry);
        debug!("Added discovered service: {}", service_id);
        Ok(())
    }

    /// Find services matching the given filter
    pub async fn find_services(&self, filter: &ServiceFilter) -> Vec<ServiceInfo> {
        let services = self.services.read().await;
        
        services
            .values()
            .filter(|entry| filter.matches(entry))
            .map(|entry| entry.service.clone())
            .collect()
    }

    /// Get all locally registered services
    pub async fn get_local_services(&self) -> Vec<ServiceInfo> {
        let filter = ServiceFilter::new().local_only();
        self.find_services(&filter).await
    }

    /// Get all discovered services
    pub async fn get_discovered_services(&self) -> Vec<ServiceInfo> {
        let filter = ServiceFilter::new().discovered_only();
        self.find_services(&filter).await
    }

    /// Get services by type
    pub async fn get_services_by_type(&self, service_type: &ServiceType) -> Vec<ServiceInfo> {
        let filter = ServiceFilter::new().with_service_types(vec![service_type.clone()]);
        self.find_services(&filter).await
    }

    /// Get services by protocol
    pub async fn get_services_by_protocol(&self, protocol: ProtocolType) -> Vec<ServiceInfo> {
        let filter = ServiceFilter::new().with_protocols(vec![protocol]);
        self.find_services(&filter).await
    }

    /// Check if a service is registered locally
    pub async fn is_local_service(&self, service_id: &str) -> bool {
        let services = self.services.read().await;
        services.get(service_id).map(|entry| entry.is_local).unwrap_or(false)
    }

    /// Check if a service exists in the registry
    pub async fn contains_service(&self, service_id: &str) -> bool {
        let services = self.services.read().await;
        services.contains_key(service_id)
    }

    /// Clean up expired services
    pub async fn cleanup_expired(&self) -> usize {
        let mut services = self.services.write().await;
        let initial_count = services.len();
        
        services.retain(|_, entry| !entry.is_expired());
        
        let removed_count = initial_count - services.len();
        if removed_count > 0 {
            debug!("Cleaned up {} expired services", removed_count);
        }
        
        removed_count
    }

    /// Get registry statistics
    pub async fn stats(&self) -> RegistryStats {
        let services = self.services.read().await;
        
        let mut local_count = 0;
        let mut discovered_count = 0;
        let mut expired_count = 0;
        
        for entry in services.values() {
            if entry.is_local {
                local_count += 1;
            } else {
                discovered_count += 1;
            }
            
            if entry.is_expired() {
                expired_count += 1;
            }
        }
        
        RegistryStats {
            total_services: services.len(),
            local_services: local_count,
            discovered_services: discovered_count,
            expired_services: expired_count,
        }
    }

    /// Find the oldest expired service for cleanup
    fn find_oldest_expired(&self, services: &HashMap<String, ServiceEntry>) -> Option<String> {
        services
            .iter()
            .filter(|(_, entry)| entry.is_expired())
            .min_by_key(|(_, entry)| entry.timestamp)
            .map(|(id, _)| id.clone())
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    /// Total number of services
    pub total_services: usize,
    /// Number of local services
    pub local_services: usize,
    /// Number of discovered services
    pub discovered_services: usize,
    /// Number of expired services
    pub expired_services: usize,
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_register_and_find_local_service() {
        let registry = ServiceRegistry::new();
        
        let service = ServiceInfo::new("test", "_http._tcp", 8080, None)
            .unwrap()
            .with_address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        
        registry.register_local_service(service.clone(), ProtocolType::Mdns).await.unwrap();
        
        let local_services = registry.get_local_services().await;
        assert_eq!(local_services.len(), 1);
        assert_eq!(local_services[0].name(), service.name());
    }

    #[tokio::test]
    async fn test_discover_and_find_service() {
        let registry = ServiceRegistry::new();
        
        let service = ServiceInfo::new("discovered", "_http._tcp", 9090, None)
            .unwrap()
            .with_address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)));
        
        registry.add_discovered_service(service.clone(), ProtocolType::Upnp, Some(Duration::from_secs(60))).await.unwrap();
        
        let discovered_services = registry.get_discovered_services().await;
        assert_eq!(discovered_services.len(), 1);
        assert_eq!(discovered_services[0].name(), service.name());
    }

    #[tokio::test]
    async fn test_service_filter() {
        let registry = ServiceRegistry::new();
        
        let http_service = ServiceInfo::new("web", "_http._tcp", 80, None)
            .unwrap()
            .with_address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        
        let ssh_service = ServiceInfo::new("ssh", "_ssh._tcp", 22, None)
            .unwrap()
            .with_address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        
        registry.register_local_service(http_service.clone(), ProtocolType::Mdns).await.unwrap();
        registry.add_discovered_service(ssh_service.clone(), ProtocolType::Upnp, Some(Duration::from_secs(60))).await.unwrap();
        
        // Test filter by type
        let http_services = registry.get_services_by_type(&ServiceType::new("_http._tcp").unwrap()).await;
        assert_eq!(http_services.len(), 1);
        assert_eq!(http_services[0].name(), "web");
        
        // Test filter by protocol
        let mdns_services = registry.get_services_by_protocol(ProtocolType::Mdns).await;
        assert_eq!(mdns_services.len(), 1);
        assert_eq!(mdns_services[0].name(), "web");
        
        // Test local only filter
        let local_services = registry.get_local_services().await;
        assert_eq!(local_services.len(), 1);
        assert_eq!(local_services[0].name(), "web");
    }

    #[tokio::test]
    async fn test_service_expiration() {
        let registry = ServiceRegistry::new();
        
        let service = ServiceInfo::new("temp", "_http._tcp", 8080, None)
            .unwrap()
            .with_address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        
        // Add service with very short TTL
        registry.add_discovered_service(service.clone(), ProtocolType::Mdns, Some(Duration::from_millis(50))).await.unwrap();
        
        // Should find service immediately
        let services = registry.get_discovered_services().await;
        assert_eq!(services.len(), 1);
        
        // Wait for expiration
        sleep(Duration::from_millis(100)).await;
        
        // Should not find expired service
        let services = registry.get_discovered_services().await;
        assert_eq!(services.len(), 0);
        
        // Cleanup should remove expired service
        let removed = registry.cleanup_expired().await;
        assert_eq!(removed, 1);
    }
}
