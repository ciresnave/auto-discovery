//! Type definitions for the auto-discovery library

use crate::service::ServiceInfo;
use crate::error::{DiscoveryError, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

/// Represents a service type for discovery
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ServiceType {
    /// The service string without protocol (e.g., "_http", "_myservice")
    service_name: String,
    /// The protocol string (e.g., "_tcp", "_udp")
    protocol: String,
    /// Optional domain for the service
    domain: Option<String>,
}

impl ServiceType {
    /// Create a new service type with default TCP protocol
    pub fn new<S: Into<String>>(service: S) -> Result<Self> {
        let service_type_str = service.into();

        if service_type_str.is_empty() {
            return Err(DiscoveryError::invalid_service("Service type cannot be empty"));
        }

        // Handle UPnP URN format (urn:schemas-upnp-org:service:ContentDirectory:1)
        if service_type_str.starts_with("urn:") {
            return Ok(ServiceType {
                service_name: service_type_str.clone(),
                protocol: "".to_string(), // UPnP doesn't use traditional protocols
                domain: None,
            });
        }

        // Parse service type like "_http._tcp.local" or "_http._tcp"
        let parts: Vec<&str> = service_type_str.split('.').collect();
        
        if parts.len() < 2 {
            return Err(DiscoveryError::invalid_service(
                "Service type must contain protocol (e.g., '._tcp')",
            ));
        }

        // Extract service name (first part)
        let service_name = parts[0].to_string();
        
        // Extract protocol (second part, should start with _)
        let protocol_part = parts[1];
        if !protocol_part.starts_with('_') {
            return Err(DiscoveryError::invalid_service(
                "Service type must contain protocol (e.g., '._tcp')",
            ));
        }
        let protocol = format!(".{protocol_part}");
        
        // Extract domain if present (third part and beyond)
        let domain = if parts.len() > 2 {
            Some(parts[2..].join("."))
        } else {
            None
        };

        // Ensure service has leading underscore
        let final_service_name = if service_name.starts_with('_') {
            service_name
        } else {
            format!("_{service_name}")
        };

        Ok(ServiceType {
            service_name: final_service_name,
            protocol,
            domain,
        })
    }

    /// Create a new service type with specified protocol
    pub fn with_protocol<S1: Into<String>, S2: Into<String>>(service: S1, protocol: S2) -> Result<Self> {
        let mut protocol_str = protocol.into();
        if &protocol_str[0..1] != "_" {
            protocol_str = format!("_{protocol_str}");
        }

        Ok(ServiceType {
            service_name: service.into(),
            protocol: protocol_str,
            domain: None,
        })
    }

    /// Create a new service type with specified domain
    pub fn with_domain<S: Into<String>>(service: S, domain: S) -> Result<Self> {
        Ok(ServiceType {
            service_name: service.into(),
            protocol: "_tcp".to_string(),
            domain: Some(domain.into()),
        })
    }

    /// Get the service string
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get the protocol string
    pub fn protocol(&self) -> &str {
        &self.protocol
    }

    /// Get the domain if present
    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }

    /// Convert to a fully qualified service string
    pub fn full_name(&self) -> String {
        // For UPnP URNs, return the service name as-is since it's already complete
        if self.service_name.starts_with("urn:") {
            return self.service_name.clone();
        }
        
        match &self.domain {
            None => format!("{}_{}", self.service_name, self.protocol),
            Some(domain) => format!("{}_{}.{}", self.service_name, self.protocol, domain),
        }
    }

    /// Check if the service type is valid
    pub fn is_valid(&self) -> bool {
        !self.service_name.is_empty() && !self.protocol.is_empty()
    }
}

impl FromStr for ServiceType {
    type Err = DiscoveryError;

    fn from_str(s: &str) -> Result<Self> {
        ServiceType::new(s)
    }
}

impl fmt::Display for ServiceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(domain) = &self.domain {
            write!(
                f,
                "{}{}.{}",
                self.service_name, self.protocol, domain
            )
        } else {
            write!(f, "{}{}", self.service_name, self.protocol)
        }
    }
}

impl From<ServiceType> for String {
    fn from(service_type: ServiceType) -> Self {
        service_type.to_string()
    }
}

/// Protocol type for service discovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ProtocolType {
    /// Multicast DNS
    #[default]
    Mdns,
    /// DNS Service Discovery
    DnsSd,
    /// Universal Plug and Play
    Upnp,
}

impl fmt::Display for ProtocolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolType::Mdns => write!(f, "mDNS"),
            ProtocolType::DnsSd => write!(f, "DNS-SD"),
            ProtocolType::Upnp => write!(f, "UPnP"),
        }
    }
}

/// Network interface information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Interface name
    pub name: String,
    /// IPv4 addresses
    pub ipv4_addresses: Vec<Ipv4Addr>,
    /// IPv6 addresses
    pub ipv6_addresses: Vec<Ipv6Addr>,
    /// Whether the interface is active
    pub is_up: bool,
    /// Whether the interface supports multicast
    pub supports_multicast: bool,
}

impl NetworkInterface {
    /// Create a new network interface
    pub fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            ipv4_addresses: Vec::new(),
            ipv6_addresses: Vec::new(),
            is_up: false,
            supports_multicast: false,
        }
    }

    /// Add an IPv4 address
    pub fn with_ipv4(mut self, addr: Ipv4Addr) -> Self {
        self.ipv4_addresses.push(addr);
        self
    }

    /// Add an IPv6 address
    pub fn with_ipv6(mut self, addr: Ipv6Addr) -> Self {
        self.ipv6_addresses.push(addr);
        self
    }

    /// Set interface status
    pub fn with_status(mut self, is_up: bool, supports_multicast: bool) -> Self {
        self.is_up = is_up;
        self.supports_multicast = supports_multicast;
        self
    }

    /// Get all IP addresses
    pub fn all_addresses(&self) -> Vec<IpAddr> {
        let mut addresses = Vec::new();
        addresses.extend(self.ipv4_addresses.iter().map(|&addr| IpAddr::V4(addr)));
        addresses.extend(self.ipv6_addresses.iter().map(|&addr| IpAddr::V6(addr)));
        addresses
    }
}

/// Service attributes as key-value pairs
pub type ServiceAttributes = HashMap<String, String>;

/// Filter for discovered services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryFilter {
    /// Service type filters
    pub service_type_filters: Vec<ServiceType>,
    /// Protocol type filters
    pub protocol_filters: Vec<ProtocolType>,
    /// Custom attribute filter patterns (key-value regex patterns)
    pub attribute_patterns: Vec<(String, String)>,
}

impl DiscoveryFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self {
            service_type_filters: Vec::new(),
            protocol_filters: Vec::new(),
            attribute_patterns: Vec::new(),
        }
    }

    /// Add a service type filter
    pub fn with_service_type(mut self, service_type: ServiceType) -> Self {
        self.service_type_filters.push(service_type);
        self
    }

    /// Add a protocol filter
    pub fn with_protocol(mut self, protocol: ProtocolType) -> Self {
        self.protocol_filters.push(protocol);
        self
    }

    /// Add an attribute pattern filter (key regex, value regex)
    pub fn with_attribute_pattern(mut self, key_pattern: String, value_pattern: String) -> Self {
        self.attribute_patterns.push((key_pattern, value_pattern));
        self
    }

    /// Check if a service matches this filter
    pub fn matches(&self, service: &ServiceInfo) -> bool {
        // Check service type filters
        if !self.service_type_filters.is_empty() 
            && !self.service_type_filters.contains(&service.service_type) {
            return false;
        }

        // Check protocol filters
        if !self.protocol_filters.is_empty() 
            && !self.protocol_filters.contains(&service.protocol_type) {
            return false;
        }

        // Check attribute pattern filters
        for (key_pattern, value_pattern) in &self.attribute_patterns {
            let mut matches = false;
            for (key, value) in &service.attributes {
                // Simple string matching for now (could be enhanced with regex)
                if key.contains(key_pattern) && value.contains(value_pattern) {
                    matches = true;
                    break;
                }
            }
            if !matches {
                return false;
            }
        }

        true
    }
}

impl Default for DiscoveryFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_type() -> Result<()> {
        let service = ServiceType::new("_http._tcp")?;
        assert_eq!(service.service_name, "_http");
        assert_eq!(service.protocol, "._tcp");
        assert_eq!(service.domain, None);
        assert_eq!(service.to_string(), "_http._tcp");
        Ok(())
    }

    #[test]
    fn test_service_type_with_domain() -> Result<()> {
        let service = ServiceType::new("_http._tcp.local")?;
        assert_eq!(service.service_name, "_http");
        assert_eq!(service.protocol, "._tcp");
        assert_eq!(service.domain, Some("local".to_string()));
        assert_eq!(service.to_string(), "_http._tcp.local");
        Ok(())
    }

    #[test]
    fn test_invalid_service_type() {
        assert!(ServiceType::new("").is_err());
        assert!(ServiceType::new("invalid").is_err());
        assert!(ServiceType::new("_http").is_err()); // Missing protocol
    }

    #[test] 
    fn test_discovery_filter() -> Result<()> {
        use crate::service::ServiceInfo;
        
        let filter = DiscoveryFilter::new()
            .with_service_type(ServiceType::new("_http._tcp")?);

        let service = ServiceInfo::new(
            "Test Service",
            "_http._tcp",
            8080,
            Some(vec![("version", "1.0")]),
        )?;

        assert!(filter.matches(&service));
        Ok(())
    }

    #[test]
    fn test_protocol_type_default() {
        assert_eq!(ProtocolType::default(), ProtocolType::Mdns);
    }
}
