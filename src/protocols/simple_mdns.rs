//! Alternative mDNS implementation using simple-mdns crate

use crate::{
    config::DiscoveryConfig,
    error::{DiscoveryError, Result},
    registry::ServiceRegistry,
    service::ServiceInfo,
    types::{ProtocolType, ServiceType},
};
use async_trait::async_trait;
use simple_mdns::{async_discovery, InstanceInformation};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
    time::Duration,
};

/// Alternative mDNS protocol implementation using simple-mdns
pub struct SimpleMdnsProtocol {
    #[allow(dead_code)]
    config: DiscoveryConfig,
    registry: Option<Arc<ServiceRegistry>>,
}

impl SimpleMdnsProtocol {
    /// Create a new simple-mdns protocol instance
    pub async fn new(config: &DiscoveryConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            registry: None,
        })
    }

    /// Convert simple-mdns InstanceInformation to ServiceInfo
    fn convert_instance_to_service_info(&self, instance: &InstanceInformation, service_type: &str) -> Result<ServiceInfo> {
        // Create service type from the parameter
        let service_type = ServiceType::new(service_type)
            .map_err(|e| DiscoveryError::mdns(&format!("Invalid service type: {}", e)))?;

        // Use first IP address if available
        let address = instance.ip_addresses.first()
            .copied()
            .unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

        // Use first port if available
        let port = instance.ports.first().copied().unwrap_or(0);

        // Convert attributes to our format
        let attributes = if instance.attributes.is_empty() {
            None
        } else {
            let attrs: Vec<(String, String)> = instance.attributes.iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            Some(attrs)
        };

        // Generate a service name (simple-mdns doesn't provide instance names)
        let service_name = format!("{}:{}", service_type.to_string(), port);

        ServiceInfo::new(
            &service_name,
            service_type.to_string(),
            port,
            attributes.as_ref().map(|v| v.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect())
        )
        .map(|s| s.with_address(address))
    }
}

#[async_trait]
impl super::DiscoveryProtocol for SimpleMdnsProtocol {
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
        let timeout_duration = timeout.unwrap_or(Duration::from_secs(5));

        for service_type in &service_types {
            let service_type_str = service_type.to_string();
            
            // Remove .local. suffix if present for simple-mdns
            let service_type_for_discovery = if service_type_str.ends_with(".local.") {
                service_type_str.trim_end_matches(".local.")
            } else {
                &service_type_str
            };

            match async_discovery::discover_service_type(service_type_for_discovery, timeout_duration).await {
                Ok(instances) => {
                    for instance in instances {
                        match self.convert_instance_to_service_info(&instance) {
                            Ok(service) => discovered_services.push(service),
                            Err(e) => {
                                tracing::warn!("Failed to convert instance to service info: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to discover services for type {}: {}", service_type_str, e);
                }
            }
        }

        // Also include locally registered services that match the requested types
        if let Some(registry) = &self.registry {
            let local_services = registry.get_local_services().await;
            for service in local_services {
                let service_type_matches = service_types.iter().any(|st| {
                    let st_str = st.to_string();
                    let service_type_str = service.service_type.to_string();
                    
                    st_str == service_type_str ||
                    format!("{}.local.", st_str) == service_type_str ||
                    st_str == format!("{}.local.", service_type_str)
                });
                
                if service_type_matches {
                    if !discovered_services.iter().any(|ds| ds.id == service.id) {
                        discovered_services.push(service.clone());
                    }
                }
            }
        }

        Ok(discovered_services)
    }

    async fn register_service(&self, service: ServiceInfo) -> Result<()> {
        // simple-mdns doesn't seem to have a built-in registration/advertising capability
        // We'll store it in the registry for local tracking
        if let Some(registry) = &self.registry {
            registry.register_local_service(service.clone(), ProtocolType::Mdns).await?;
        }
        
        // For now, just log that we would advertise this service
        tracing::info!("Service registered locally (simple-mdns doesn't support advertising): {}", service.name);
        Ok(())
    }

    async fn unregister_service(&self, service: &ServiceInfo) -> Result<()> {
        if let Some(registry) = &self.registry {
            let service_id = format!("{}:{}:{}", service.name, service.service_type.to_string(), service.port);
            registry.unregister_local_service(&service_id).await?;
        }
        
        tracing::info!("Service unregistered locally: {}", service.name);
        Ok(())
    }

    async fn verify_service(&self, service: &ServiceInfo) -> Result<bool> {
        if let Some(registry) = &self.registry {
            let local_services = registry.get_local_services().await;
            Ok(local_services.iter().any(|s| s.name == service.name))
        } else {
            Ok(false)
        }
    }

    async fn is_available(&self) -> bool {
        // simple-mdns should be available on all platforms
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_simple_mdns_protocol() {
        let config = crate::config::DiscoveryConfig::new();
        let mut protocol = SimpleMdnsProtocol::new(&config).await.unwrap();
        
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

        // Register service
        protocol.register_service(service.clone()).await.unwrap();

        // Verify service is alive
        assert!(protocol.verify_service(&service).await.unwrap());

        // Unregister service
        protocol.unregister_service(&service).await.unwrap();

        // Verify service is no longer registered
        assert!(!protocol.verify_service(&service).await.unwrap());
    }
}
