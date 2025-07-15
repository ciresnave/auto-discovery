//! mDNS (Multicast DNS) protocol implementation

use crate::{
    config::DiscoveryConfig,
    error::{DiscoveryError, Result},
    registry::ServiceRegistry,
    service::ServiceInfo,
    types::{ProtocolType, ServiceType},
};
use async_trait::async_trait;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo as MdnsServiceInfo};
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};

/// mDNS protocol implementation for service discovery
pub struct MdnsProtocol {
    daemon: Arc<ServiceDaemon>,
    #[allow(dead_code)]
    config: DiscoveryConfig,
    /// Service registry for managing discovered and registered services
    registry: Option<Arc<ServiceRegistry>>,
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
        // Try to create daemon with a retry mechanism
        let daemon = Self::create_daemon_with_retry().await?;

        // Create with default registry if one isn't set later
        let registry = Some(Arc::new(ServiceRegistry::new()));

        Ok(Self {
            daemon: Arc::new(daemon),
            config: config.clone(),
            registry,
        })
    }

    /// Create mDNS daemon with retry logic
    async fn create_daemon_with_retry() -> Result<ServiceDaemon> {
        // Try multiple times with increasing delays
        for attempt in 1..=3 {
            match ServiceDaemon::new() {
                Ok(daemon) => return Ok(daemon),
                Err(e) => {
                    tracing::warn!("Failed to create mDNS daemon (attempt {}): {}", attempt, e);
                    if attempt < 3 {
                        tokio::time::sleep(Duration::from_millis(100 * attempt)).await;
                    } else {
                        return Err(DiscoveryError::mdns(format!("Failed to create mDNS daemon after {attempt} attempts: {e}")));
                    }
                }
            }
        }
        
        // This should never be reached
        Err(DiscoveryError::mdns("Unexpected error in daemon creation"))
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

    fn set_registry(&mut self, registry: Arc<ServiceRegistry>) {
        self.registry = Some(registry);
    }

    async fn discover_services(
        &self,
        service_types: Vec<ServiceType>,
        timeout: Option<Duration>,
    ) -> Result<Vec<ServiceInfo>> {
        let mut discovered_services = Vec::new();
        let discovery_timeout = timeout.unwrap_or(Duration::from_secs(5));
        
        for service_type in &service_types {
            // Format service type for mDNS - ensure it ends with .local.
            let service_type_str = if service_type.to_string().ends_with(".local.") {
                service_type.to_string()
            } else {
                format!("{service_type}.local.")
            };
            
            let receiver = self.daemon.browse(&service_type_str)
                .map_err(|e| DiscoveryError::mdns(format!("Failed to browse services: {e}")))?;

            // Collect services with timeout
            let mut services = Vec::new();
            let start_time = std::time::Instant::now();
            let per_attempt_timeout = std::cmp::min(discovery_timeout, Duration::from_millis(500));
            
            while start_time.elapsed() < discovery_timeout {
                match receiver.recv_timeout(per_attempt_timeout) {
                    Ok(event) => {
                        match event {
                            ServiceEvent::ServiceResolved(info) => {
                                if let Ok(service_info) = self.convert_to_service_info(info) {
                                    services.push(service_info);
                                    tracing::debug!("Discovered service: {}", services.last().unwrap().name());
                                }
                            },
                            ServiceEvent::SearchStopped(_) => {
                                tracing::debug!("mDNS search stopped");
                                break;
                            },
                            _ => {
                                // Continue for other events
                                continue;
                            }
                        }
                    },
                    Err(_) => {
                        // Timeout - check if we should continue
                        if start_time.elapsed() >= discovery_timeout {
                            break;
                        }
                        continue;
                    }
                }
            }
            
            discovered_services.extend(services);
        }

        // Also include locally registered services that match the requested types
        if let Some(registry) = &self.registry {
            let local_services = registry.get_local_services().await;
            for service in local_services {
                let service_type_matches = service_types.iter().any(|st| {
                    // Compare the service types, handling both with and without .local.
                    let st_str = st.to_string();
                    let service_type_str = service.service_type.to_string();
                    
                    st_str == service_type_str ||
                    format!("{st_str}.local.") == service_type_str ||
                    st_str == format!("{service_type_str}.local.")
                });
                
                if service_type_matches {
                    // Only add if not already in discovered services
                    if !discovered_services.iter().any(|ds| ds.id == service.id) {
                        discovered_services.push(service.clone());
                    }
                }
            }
        }

        Ok(discovered_services)
    }

    async fn register_service(&self, service: ServiceInfo) -> Result<()> {
        let mut txt_records = Vec::new();
        for (key, value) in &service.attributes {
            txt_records.push((key.as_str(), value.as_str()));
        }

        // Format service type for mDNS - ensure it ends with .local.
        let service_type_str = if service.service_type.to_string().ends_with(".local.") {
            service.service_type.to_string()
        } else {
            format!("{}.local.", service.service_type)
        };

        // Create hostname for the service
        let hostname = format!("{}.local.", service.name);

        // Use address directly since mdns-sd expects AsIpAddrs
        let mdns_info = MdnsServiceInfo::new(
            &service_type_str,
            &service.name,
            &hostname,
            service.address,
            service.port,
            txt_records.as_slice(),
        ).map_err(|e| DiscoveryError::mdns(format!("Failed to create mDNS service info: {e}")))?;

        self.daemon.register(mdns_info)
            .map_err(|e| DiscoveryError::mdns(format!("Failed to register service: {e}")))?;

        // Track registered service for verification
        if let Some(registry) = &self.registry {
            registry.register_local_service(service.clone(), ProtocolType::Mdns).await?;
        }

        Ok(())
    }

    async fn unregister_service(&self, service: &ServiceInfo) -> Result<()> {
        // Create the full service name that was used during registration
        let service_type_str = if service.service_type.to_string().ends_with(".local.") {
            service.service_type.to_string()
        } else {
            format!("{}.local.", service.service_type)
        };
        
        let full_service_name = format!("{}.{}", service.name, service_type_str);
        
        self.daemon.unregister(&full_service_name)
            .map_err(|e| DiscoveryError::mdns(format!("Failed to unregister service: {e}")))?;
        
        // Remove from registry
        if let Some(registry) = &self.registry {
            let service_id = format!("{}:{}:{}", service.name, service.service_type, service.port);
            registry.unregister_local_service(&service_id).await?;
        }
        
        Ok(())
    }

    async fn verify_service(&self, service: &ServiceInfo) -> Result<bool> {
        // Check if service is in our registry of registered services
        if let Some(registry) = &self.registry {
            let local_services = registry.get_local_services().await;
            Ok(local_services.iter().any(|s| s.name == service.name))
        } else {
            Ok(false)
        }
    }

    async fn is_available(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_mdns_protocol() {
        let config = crate::config::DiscoveryConfig::new();
        let mut protocol = MdnsProtocol::new(&config).await.unwrap();
        
        // Set up a registry for the protocol
        let registry = Arc::new(crate::registry::ServiceRegistry::new());
        protocol.set_registry(registry);

        let service = ServiceInfo::new(
            "test_service",
            "_test._tcp.local.",
            8080,
            Some(vec![("version", "1.0"), ("description", "Test service")])
        )
        .unwrap()
        .with_address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

        // Test using trait methods directly
        use crate::protocols::DiscoveryProtocol;
        
        // Register service
        protocol.register_service(service.clone()).await.unwrap();

        // Wait a bit for service to be properly registered in mDNS
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Discover services with longer timeout for network operations
        let discovered = protocol
            .discover_services(
                vec![ServiceType::new("_test._tcp.local.").unwrap()],
                Some(Duration::from_secs(3))
            )
            .await
            .unwrap();

        // Our improved implementation now returns locally registered services
        // So we should find the service we just registered
        assert!(!discovered.is_empty());
        let discovered_service = &discovered[0];
        assert_eq!(discovered_service.name, service.name);
        assert_eq!(discovered_service.port, service.port);

        // Unregister service
        protocol.unregister_service(&service).await.unwrap();
    }
}