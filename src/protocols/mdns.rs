//! mDNS (Multicast DNS) protocol implementation

use crate::{
    config::DiscoveryConfig,
    error::{DiscoveryError, Result},
    service::ServiceInfo,
    types::{ProtocolType, ServiceType},
};
use async_trait::async_trait;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo as MdnsServiceInfo};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

/// mDNS protocol implementation for service discovery
pub struct MdnsProtocol {
    daemon: Arc<ServiceDaemon>,
    #[allow(dead_code)]
    config: DiscoveryConfig,
}

impl MdnsProtocol {
    /// Create a new mDNS protocol instance
    /// 
    /// # Arguments
    /// 
    /// * `config` - The discovery configuration to use
    /// 
    /// # Errors
    /// 
    /// Returns an error if the mDNS daemon cannot be initialized
    pub async fn new(config: &DiscoveryConfig) -> Result<Self> {
        let daemon = ServiceDaemon::new()
            .map_err(|e| DiscoveryError::mdns(&format!("Failed to create mDNS daemon: {}", e)))?;

        Ok(Self {
            daemon: Arc::new(daemon),
            config: config.clone(),
        })
    }

    #[allow(dead_code)]
    fn convert_to_service_info(&self, mdns_info: MdnsServiceInfo) -> Result<ServiceInfo> {
        let host = mdns_info.get_hostname().to_string();
        let service_type = ServiceType::new(mdns_info.get_type())?;
        let addresses = mdns_info.get_addresses();
        let port = mdns_info.get_port();

        if addresses.is_empty() {
            return Err(DiscoveryError::mdns("Service has no addresses"));
        }

        // Convert TXT records to attributes (simplified)
        let attributes: HashMap<String, String> = HashMap::new(); // For now, skip TXT record parsing

        let mut service = ServiceInfo::new(
            host,
            service_type,
            port,
            None,
        )?;

        service = service
            .with_protocol_type(ProtocolType::Mdns)
            .with_address(*addresses.iter().next().unwrap())
            .with_attributes(attributes);

        Ok(service)
    }
}

#[async_trait]
impl super::DiscoveryProtocol for MdnsProtocol {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::Mdns
    }

    async fn discover_services(
        &self,
        service_types: Vec<ServiceType>,
        _timeout: Option<Duration>,
    ) -> Result<Vec<ServiceInfo>> {
        let services = Vec::new();
        
        for service_type in service_types {
            let receiver = self.daemon.browse(&service_type.to_string())
                .map_err(|e| DiscoveryError::mdns(&format!("Failed to browse services: {}", e)))?;

            // For a real implementation, we would collect services from the receiver
            // This is a placeholder that should be improved
            tokio::spawn(async move {
                while let Ok(event) = receiver.recv() {
                    match event {
                        ServiceEvent::ServiceFound(_, _) => (),
                        ServiceEvent::ServiceResolved(_info) => {
                            // Process resolved service info
                        },
                        ServiceEvent::ServiceRemoved(_, _) => (),
                        ServiceEvent::SearchStarted(_) => (),
                        ServiceEvent::SearchStopped(_) => (),
                    }
                }
            });
        }

        Ok(services)
    }

    async fn register_service(&self, service: ServiceInfo) -> Result<()> {
        let mut txt_records = Vec::new();
        for (key, value) in &service.attributes {
            txt_records.push((key.as_str(), value.as_str()));
        }

        // Use address directly since mdns-sd expects AsIpAddrs
        let mdns_info = MdnsServiceInfo::new(
            &service.service_type.to_string(),
            &service.name,
            &service.address.to_string(),
            service.address,
            service.port,
            txt_records.as_slice(),
        ).map_err(|e| DiscoveryError::mdns(&format!("Failed to create mDNS service info: {}", e)))?;

        self.daemon.register(mdns_info)
            .map_err(|e| DiscoveryError::mdns(&format!("Failed to register service: {}", e)))?;

        Ok(())
    }

    async fn unregister_service(&self, service: &ServiceInfo) -> Result<()> {
        self.daemon.unregister(&service.name)
            .map_err(|e| DiscoveryError::mdns(&format!("Failed to unregister service: {}", e)))?;
        Ok(())
    }

    async fn verify_service(&self, _service: &ServiceInfo) -> Result<bool> {
        // Simple availability check - could be enhanced
        Ok(true)
    }

    async fn is_available(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_mdns_protocol() {
        let config = crate::config::DiscoveryConfig::new();
        let protocol = MdnsProtocol::new(&config).await.unwrap();

        let service = ServiceInfo::new(
            "test_service._test._tcp.local.",
            "_test._tcp.local",
            8080,
            Some(vec![("version", "1.0"), ("description", "Test service")])
        )
        .unwrap()
        .with_address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

        // Test using trait methods directly
        use crate::protocols::DiscoveryProtocol;
        
        // Register service
        protocol.register_service(service.clone()).await.unwrap();

        // Discover services
        let discovered = protocol
            .discover_services(
                vec![ServiceType::new("_test._tcp.local").unwrap()],
                Some(Duration::from_secs(1))
            )
            .await
            .unwrap();

        assert!(!discovered.is_empty());
        let discovered_service = &discovered[0];
        assert_eq!(discovered_service.name(), service.name());
        assert_eq!(discovered_service.port(), service.port());

        // Unregister service
        protocol.unregister_service(&service).await.unwrap();
    }
}
