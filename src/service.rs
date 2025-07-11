//! Service information and event types

use crate::types::{ProtocolType, ServiceAttributes, ServiceType};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt,
    net::{IpAddr, Ipv4Addr},
    time::{Duration, SystemTime},
};
use uuid::Uuid;

/// ServiceInfo holds information about a discovered or registered service
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceInfo {
    /// Unique identifier for this service instance
    pub id: Uuid,
    /// Human-readable name of the service
    pub name: String,
    /// Service type (e.g., "_http._tcp")
    pub service_type: ServiceType,
    /// IP address of the service
    pub address: IpAddr,
    /// Port number of the service
    pub port: u16,
    /// Additional service attributes
    pub attributes: ServiceAttributes,
    /// Protocol used to discover this service
    pub protocol_type: ProtocolType,
    /// Time when the service was discovered
    pub discovered_at: SystemTime,
    /// Time-to-live for the service record
    pub ttl: Duration,
    /// Whether the service has been verified
    pub verified: bool,
    /// Network interface name where the service was discovered
    pub interface: Option<String>,
}

impl ServiceInfo {
    /// Create a new service info
    pub fn new(
        name: impl Into<String>,
        service_type: impl Into<String>,
        port: u16,
        attributes: Option<Vec<(&str, &str)>>
    ) -> Result<Self, crate::error::DiscoveryError> {
        let service_type = ServiceType::new(service_type)?;
        
        let mut info = Self {
            id: Uuid::new_v4(),
            name: name.into(),
            service_type,
            address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port,
            attributes: HashMap::new(),
            protocol_type: ProtocolType::default(),
            discovered_at: SystemTime::now(),
            ttl: Duration::from_secs(60),
            verified: false,
            interface: None,
        };

        if let Some(attrs) = attributes {
            for (key, value) in attrs {
                info.attributes.insert(key.to_string(), value.to_string());
            }
        }

        Ok(info)
    }

    /// Get protocol type used for this service
    pub fn protocol_type(&self) -> ProtocolType {
        self.protocol_type
    }

    /// Set protocol type
    pub fn with_protocol_type(mut self, protocol_type: ProtocolType) -> Self {
        self.protocol_type = protocol_type;
        self
    }

    /// Get service TTL
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Set service TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Check if service has expired
    pub fn is_expired(&self) -> bool {
        match self.discovered_at.elapsed() {
            Ok(elapsed) => elapsed > self.ttl,
            Err(_) => false,
        }
    }

    /// Refresh service discovery time
    pub fn refresh(&mut self) {
        self.discovered_at = SystemTime::now();
    }

    /// Set service attributes
    pub fn with_attributes<K, V>(mut self, attrs: HashMap<K, V>) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.attributes = attrs
            .into_iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        self
    }

    /// Set a single attribute
    pub fn with_attribute<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Insert or update an attribute
    pub fn insert_attribute<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Get an attribute value
    pub fn get_attribute(&self, key: &str) -> Option<&String> {
        self.attributes.get(key)
    }

    /// Get service port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get service address
    pub fn address(&self) -> IpAddr {
        self.address
    }

    /// Set service address
    pub fn with_address(mut self, address: IpAddr) -> Self {
        self.address = address;
        self
    }

    /// Get service name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get service type
    pub fn service_type(&self) -> &ServiceType {
        &self.service_type
    }
}

impl fmt::Display for ServiceInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}) at {}:{} via {}",
            self.name, self.service_type, self.address, self.port, self.protocol_type
        )
    }
}

/// Events that can occur during service discovery
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceEvent {
    /// A new service was discovered
    New(ServiceInfo),
    /// An existing service was updated
    Updated(ServiceInfo),
    /// A service was removed or expired
    Removed(ServiceInfo),
    /// A service failed verification
    VerificationFailed(ServiceInfo),
    /// Discovery process started
    DiscoveryStarted {
        /// Service types being searched for
        service_types: Vec<ServiceType>,
        /// Protocols being used
        protocols: Vec<ProtocolType>,
    },
    /// Discovery process completed
    DiscoveryCompleted {
        /// Number of services found
        services_found: usize,
        /// Time taken for discovery
        duration: Duration,
    },
    /// Discovery process failed
    DiscoveryFailed {
        /// Error message
        error: String,
        /// Service types that failed
        service_types: Vec<ServiceType>,
    },
}

impl ServiceEvent {
    /// Create a new service event
    pub fn new(service: ServiceInfo) -> Self {
        Self::New(service)
    }

    /// Create an updated service event
    pub fn updated(service: ServiceInfo) -> Self {
        Self::Updated(service)
    }

    /// Create a removed service event
    pub fn removed(service: ServiceInfo) -> Self {
        Self::Removed(service)
    }

    /// Create a verification failed event
    pub fn verification_failed(service: ServiceInfo) -> Self {
        Self::VerificationFailed(service)
    }

    /// Create a discovery started event
    pub fn discovery_started(
        service_types: Vec<ServiceType>,
        protocols: Vec<ProtocolType>,
    ) -> Self {
        Self::DiscoveryStarted {
            service_types,
            protocols,
        }
    }

    /// Create a discovery completed event
    pub fn discovery_completed(services_found: usize, duration: Duration) -> Self {
        Self::DiscoveryCompleted {
            services_found,
            duration,
        }
    }

    /// Create a discovery failed event
    pub fn discovery_failed<S: Into<String>>(error: S, service_types: Vec<ServiceType>) -> Self {
        Self::DiscoveryFailed {
            error: error.into(),
            service_types,
        }
    }

    /// Get the service info if this event contains one
    pub fn service(&self) -> Option<&ServiceInfo> {
        match self {
            Self::New(service)
            | Self::Updated(service)
            | Self::Removed(service)
            | Self::VerificationFailed(service) => Some(service),
            _ => None,
        }
    }

    /// Check if this is a positive event (new or updated service)
    pub fn is_positive(&self) -> bool {
        matches!(self, Self::New(_) | Self::Updated(_) | Self::DiscoveryCompleted { .. })
    }

    /// Check if this is a negative event (removed service or failure)
    pub fn is_negative(&self) -> bool {
        matches!(
            self,
            Self::Removed(_) | Self::VerificationFailed(_) | Self::DiscoveryFailed { .. }
        )
    }
}

impl fmt::Display for ServiceEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::New(service) => write!(f, "New service: {}", service),
            Self::Updated(service) => write!(f, "Updated service: {}", service),
            Self::Removed(service) => write!(f, "Removed service: {}", service),
            Self::VerificationFailed(service) => write!(f, "Verification failed: {}", service),
            Self::DiscoveryStarted {
                service_types,
                protocols,
            } => write!(
                f,
                "Discovery started for {} service types using {} protocols",
                service_types.len(),
                protocols.len()
            ),
            Self::DiscoveryCompleted {
                services_found,
                duration,
            } => write!(
                f,
                "Discovery completed: {} services found in {:?}",
                services_found, duration
            ),
            Self::DiscoveryFailed { error, service_types } => write!(
                f,
                "Discovery failed for {} service types: {}",
                service_types.len(),
                error
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ProtocolType;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_service_creation() -> Result<(), crate::error::DiscoveryError> {
        let service = ServiceInfo::new(
            "Test Service",
            "_http._tcp",
            8080,
            Some(vec![("version", "1.0"), ("protocol", "HTTP/1.1")]),
        )?;

        assert_eq!(service.name, "Test Service");
        assert_eq!(service.get_attribute("version"), Some(&"1.0".to_string()));
        assert!(!service.is_expired());

        Ok(())
    }

    #[test]
    fn test_service_expiry() -> Result<(), crate::error::DiscoveryError> {
        let mut service = ServiceInfo::new(
            "Test Service",
            "_http._tcp",
            8080,
            None,
        )?
        .with_ttl(Duration::from_nanos(1));

        std::thread::sleep(Duration::from_millis(1));
        assert!(service.is_expired());

        service.refresh();
        assert!(!service.is_expired());

        Ok(())
    }

    #[test]
    fn test_service_attributes() -> Result<(), crate::error::DiscoveryError> {
        let service = ServiceInfo::new("Test Service", "_http._tcp", 8080, None)?
            .with_attribute("version", "1.0")
            .with_attribute("secure", "true");

        assert_eq!(service.get_attribute("version"), Some(&"1.0".to_string()));
        assert_eq!(service.get_attribute("secure"), Some(&"true".to_string()));
        assert_eq!(service.get_attribute("nonexistent"), None);

        Ok(())
    }

    #[test]
    fn test_service_protocol() -> Result<(), crate::error::DiscoveryError> {
        let service = ServiceInfo::new("Test Service", "_http._tcp", 8080, None)?
            .with_protocol_type(ProtocolType::Mdns);

        assert_eq!(service.protocol_type(), ProtocolType::Mdns);
        Ok(())
    }
}
