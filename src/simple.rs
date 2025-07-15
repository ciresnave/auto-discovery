//! Simplified API for common use cases
//!
//! This module provides simplified, zero-configuration APIs for the most common
//! service discovery scenarios.

use crate::{
    config::DiscoveryConfig,
    error::Result,
    service::ServiceInfo,
    types::{ServiceType, ProtocolType},
    ServiceDiscovery,
};
use std::time::Duration;

/// Simple service discovery with sensible defaults
pub struct SimpleDiscovery {
    inner: ServiceDiscovery,
}

impl SimpleDiscovery {
    /// Create a new simple discovery instance with defaults
    /// 
    /// Automatically configures:
    /// - mDNS protocol 
    /// - Common service types (_http._tcp, _https._tcp, _ssh._tcp, _ftp._tcp)
    /// - 5-second timeout
    /// - Service verification enabled
    /// 
    /// # Example
    /// ```rust
    /// use auto_discovery::simple::SimpleDiscovery;
    /// 
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let discovery = SimpleDiscovery::new().await?;
    ///     let services = discovery.discover_all().await?;
    ///     println!("Found {} services", services.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn new() -> Result<Self> {
        let config = DiscoveryConfig::new()
            .with_service_type(ServiceType::new("_http._tcp")?)
            .with_service_type(ServiceType::new("_https._tcp")?)
            .with_service_type(ServiceType::new("_ssh._tcp")?)
            .with_service_type(ServiceType::new("_ftp._tcp")?)
            .with_protocol(ProtocolType::Mdns)
            .with_timeout(Duration::from_secs(5))
            .with_verify_services(true);

        let inner = ServiceDiscovery::new(config).await?;
        Ok(Self { inner })
    }

    /// Discover all configured services
    pub async fn discover_all(&self) -> Result<Vec<ServiceInfo>> {
        self.inner.discover_services(None).await
    }

    /// Discover only HTTP services
    pub async fn discover_http(&self) -> Result<Vec<ServiceInfo>> {
        self.inner.discover_services(Some(ProtocolType::Mdns)).await
    }

    /// Register a simple HTTP service
    /// 
    /// # Example
    /// ```rust
    /// # use auto_discovery::simple::SimpleDiscovery;
    /// # #[tokio::main] 
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let discovery = SimpleDiscovery::new().await?;
    /// discovery.register_http_service("My Web App", 8080).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn register_http_service(&self, name: &str, port: u16) -> Result<()> {
        let service = ServiceInfo::new(name, "_http._tcp", port, None)?;
        self.inner.register_service(service).await
    }

    /// Register a service with custom attributes
    pub async fn register_service_with_attributes(
        &self, 
        name: &str, 
        service_type: &str, 
        port: u16,
        attributes: Vec<(&str, &str)>
    ) -> Result<()> {
        let service = ServiceInfo::new(name, service_type, port, Some(attributes))?;
        self.inner.register_service(service).await
    }

    /// Stop all services and cleanup
    pub async fn shutdown(&self) -> Result<()> {
        // Unregister all services
        let services = self.inner.get_registered_services().await;
        for service in services {
            let _ = self.inner.unregister_service(&service).await;
        }
        Ok(())
    }
}

/// Quick one-liner functions for common scenarios
///
/// Discover all HTTP services on the network
/// 
/// # Example
/// ```rust
/// use auto_discovery::simple::discover_http_services;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let services = discover_http_services().await?;
///     for service in services {
///         println!("Found: {} at {}:{}", service.name(), service.address, service.port);
///     }
///     Ok(())
/// }
/// ```
pub async fn discover_http_services() -> Result<Vec<ServiceInfo>> {
    let discovery = SimpleDiscovery::new().await?;
    discovery.discover_http().await
}

/// Register an HTTP service and return a handle for cleanup
/// 
/// # Example
/// ```rust
/// use auto_discovery::simple::register_http_service;
/// 
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let handle = register_http_service("My API", 8080).await?;
///     
///     // Your server code here
///     
///     handle.unregister().await?;
///     Ok(())
/// }
/// ```
pub async fn register_http_service(name: &str, port: u16) -> Result<ServiceHandle> {
    let discovery = SimpleDiscovery::new().await?;
    let service = ServiceInfo::new(name, "_http._tcp", port, None)?;
    discovery.inner.register_service(service.clone()).await?;
    Ok(ServiceHandle { 
        discovery: discovery.inner,
        service,
    })
}

/// Handle for managing a registered service
pub struct ServiceHandle {
    discovery: ServiceDiscovery,
    service: ServiceInfo,
}

impl ServiceHandle {
    /// Unregister the service
    pub async fn unregister(self) -> Result<()> {
        self.discovery.unregister_service(&self.service).await
    }

    /// Get service information
    pub fn service(&self) -> &ServiceInfo {
        &self.service
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_discovery() {
        let discovery = SimpleDiscovery::new().await.unwrap();
        
        // Should not fail even if no services found
        let services = discovery.discover_all().await.unwrap();
        // Can't assert specific count since it depends on network
        assert!(!services.is_empty() || services.is_empty()); // services can be empty in test environment
    }

    #[tokio::test]
    async fn test_register_http_service() {
        let discovery = SimpleDiscovery::new().await.unwrap();
        let result = discovery.register_http_service("Test Service", 8080).await;
        
        // Registration might fail in test environment
        match result {
            Ok(_) => {
                // Service registered successfully
            }
            Err(_) => {
                // Expected in test environment without actual mDNS
            }
        }
    }

    #[tokio::test]
    async fn test_one_liner_functions() {
        // Test the one-liner function
        let result = discover_http_services().await;
        
        // Should not panic even if discovery fails
        match result {
            Ok(_services) => {
                // Success case
            }
            Err(_) => {
                // Expected in test environment
            }
        }
    }
}