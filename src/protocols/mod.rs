//! Protocol implementations for service discovery

use crate::{
    config::DiscoveryConfig,
    error::{DiscoveryError, Result},
    service::ServiceInfo,
    types::{ProtocolType, ServiceType},
};
use async_trait::async_trait;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::warn;

pub mod mdns;
pub mod upnp;
pub mod dns_sd;

/// Trait for service discovery protocols
#[async_trait]
pub trait DiscoveryProtocol: Send + Sync {
    /// Get the protocol type
    fn protocol_type(&self) -> ProtocolType;

    /// Discover services of the specified types with timeout
    async fn discover_services(
        &self,
        service_types: Vec<ServiceType>,
        timeout: Option<Duration>,
    ) -> Result<Vec<ServiceInfo>>;

    /// Register a service for advertisement
    async fn register_service(&self, service: ServiceInfo) -> Result<()>;

    /// Unregister a service
    async fn unregister_service(&self, service: &ServiceInfo) -> Result<()>;

    /// Verify if a service is alive and healthy
    async fn verify_service(&self, service: &ServiceInfo) -> Result<bool>;

    /// Check if the protocol is available
    async fn is_available(&self) -> bool;
}

/// Manager for all discovery protocols
#[derive(Clone)]
pub struct ProtocolManager {
    #[allow(dead_code)]
    config: DiscoveryConfig,
    protocols: HashMap<ProtocolType, Arc<dyn DiscoveryProtocol + Send + Sync>>,
}

impl ProtocolManager {
    /// Create a new protocol manager
    pub async fn new(config: DiscoveryConfig) -> Result<Self> {
        let mut protocols: HashMap<ProtocolType, Arc<dyn DiscoveryProtocol + Send + Sync>> = HashMap::new();

        // Initialize protocols based on config
        if config.has_protocol(ProtocolType::Mdns) {
            if let Ok(mdns) = mdns::MdnsProtocol::new(&config).await {
                protocols.insert(ProtocolType::Mdns, Arc::new(mdns) as Arc<dyn DiscoveryProtocol + Send + Sync>);
            }
        }

        if config.has_protocol(ProtocolType::Upnp) {
            if let Ok(ssdp) = upnp::SsdpProtocol::new(&config).await {
                protocols.insert(ProtocolType::Upnp, Arc::new(ssdp) as Arc<dyn DiscoveryProtocol + Send + Sync>);
            }
        }

        if config.has_protocol(ProtocolType::DnsSd) {
            if let Ok(dns_sd) = dns_sd::DnsSdProtocol::new(&config).await {
                protocols.insert(ProtocolType::DnsSd, Arc::new(dns_sd) as Arc<dyn DiscoveryProtocol + Send + Sync>);
            }
        }

        Ok(Self { config, protocols })
    }

    /// Get enabled protocol types
    pub fn protocol_types(&self) -> Vec<ProtocolType> {
        self.protocols.keys().copied().collect()
    }

    /// Discover services with all enabled protocols
    pub async fn discover_services(
        &self,
        service_types: Vec<ServiceType>,
        timeout: Option<Duration>,
    ) -> Result<Vec<ServiceInfo>> {
        let mut all_services = Vec::new();

        for protocol in self.protocols.values() {
            match protocol.discover_services(service_types.clone(), timeout).await {
                Ok(services) => all_services.extend(services),
                Err(e) => warn!(
                    "Error discovering services with protocol {:?}: {}",
                    protocol.protocol_type(),
                    e
                ),
            }
        }

        Ok(all_services)
    }

    /// Discover services with a specific protocol
    pub async fn discover_services_with_protocol(
        &self,
        protocol_type: ProtocolType,
        service_types: Vec<ServiceType>,
        timeout: Option<Duration>,
    ) -> Result<Vec<ServiceInfo>> {
        if let Some(protocol) = self.protocols.get(&protocol_type) {
            return protocol.discover_services(service_types, timeout).await;
        }
        Err(DiscoveryError::protocol(format!("Protocol {:?} not available", protocol_type)))
    }

    /// Register a service with the appropriate protocol
    pub async fn register_service(&self, service: ServiceInfo) -> Result<()> {
        let protocol_type = service.protocol_type();
        if let Some(protocol) = self.protocols.get(&protocol_type) {
            return protocol.register_service(service).await;
        }
        
        Err(DiscoveryError::protocol(format!(
            "Protocol {:?} not available",
            protocol_type
        )))
    }

    /// Unregister a service
    pub async fn unregister_service(&self, service: &ServiceInfo) -> Result<()> {
        let protocol_type = service.protocol_type();
        if let Some(protocol) = self.protocols.get(&protocol_type) {
            return protocol.unregister_service(service).await;
        }
        return Err(DiscoveryError::protocol(format!(
            "Protocol {:?} not available",
            protocol_type
        )));
    }

    /// Verify a service is still available
    pub async fn verify_service(&self, service: &ServiceInfo) -> Result<bool> {
        let protocol_type = service.protocol_type();
        if let Some(protocol) = self.protocols.get(&protocol_type) {
            return protocol.verify_service(service).await;
        }
        return Err(DiscoveryError::protocol(format!(
            "Protocol {:?} not available",
            protocol_type
        )));
    }

    /// Get a reference to the protocols map
    pub fn protocols(&self) -> &HashMap<ProtocolType, Arc<dyn DiscoveryProtocol + Send + Sync>> {
        &self.protocols
    }

    /// Perform a health check on all protocols
    pub async fn health_check(&self) -> HashMap<ProtocolType, bool> {
        let mut statuses = HashMap::new();
        for (protocol_type, protocol) in &self.protocols {
            statuses.insert(*protocol_type, protocol.is_available().await);
        }
        statuses
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DiscoveryConfig;

    #[tokio::test]
    async fn test_protocol_manager_creation() {
        let config = DiscoveryConfig::new();
        let manager = ProtocolManager::new(config).await;

        // This might fail in test environment without proper network setup
        match manager {
            Ok(manager) => {
                assert!(!manager.protocols.is_empty());
            }
            Err(_) => {
                // Expected in some test environments
            }
        }
    }

    #[tokio::test]
    async fn test_protocol_availability() {
        let config = DiscoveryConfig::new().with_protocol(ProtocolType::Mdns);
        if let Ok(manager) = ProtocolManager::new(config).await {
            let protocols = manager.protocol_types();
            assert!(!protocols.is_empty());
        }
    }

    #[tokio::test]
    async fn test_service_registration() {
        let config = DiscoveryConfig::new().with_protocol(ProtocolType::Mdns);
        let manager = ProtocolManager::new(config).await.unwrap();

        let service = ServiceInfo::new(
            "test_service",
            "_http._tcp",
            8080,
            Some(vec![("version", "1.0")])
        )
        .unwrap()
        .with_protocol_type(ProtocolType::Mdns);

        assert!(manager.register_service(service).await.is_ok());
    }
}
