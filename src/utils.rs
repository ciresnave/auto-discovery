//! Utility functions for the auto-discovery library

use crate::{
    error::{DiscoveryError, Result},
    types::NetworkInterface,
};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tracing::{debug, warn};

/// Network utility functions
pub mod network {
    use super::*;

    /// Get all available network interfaces on the system
    pub fn get_network_interfaces() -> Result<Vec<NetworkInterface>> {
        debug!("Enumerating network interfaces");

        // This is a placeholder implementation
        // In a real implementation, you would use platform-specific APIs
        // or a cross-platform crate like `pnet` or `nix`

        let mut interfaces = Vec::new();

        // Create mock interfaces for demonstration
        let localhost = NetworkInterface::new("lo")
            .with_ipv4(Ipv4Addr::LOCALHOST)
            .with_ipv6(Ipv6Addr::LOCALHOST)
            .with_status(true, false);
        interfaces.push(localhost);

        #[cfg(target_os = "windows")]
        {
            let ethernet = NetworkInterface::new("Ethernet")
                .with_ipv4("192.168.1.100".parse().unwrap())
                .with_ipv6("fe80::1".parse().unwrap())
                .with_status(true, true);
            interfaces.push(ethernet);

            let wifi = NetworkInterface::new("Wi-Fi")
                .with_ipv4("192.168.1.101".parse().unwrap())
                .with_ipv6("fe80::2".parse().unwrap())
                .with_status(true, true);
            interfaces.push(wifi);
        }

        #[cfg(target_os = "linux")]
        {
            let eth0 = NetworkInterface::new("eth0")
                .with_ipv4("192.168.1.100".parse().unwrap())
                .with_ipv6("fe80::1".parse().unwrap())
                .with_status(true, true);
            interfaces.push(eth0);

            let wlan0 = NetworkInterface::new("wlan0")
                .with_ipv4("192.168.1.101".parse().unwrap())
                .with_ipv6("fe80::2".parse().unwrap())
                .with_status(true, true);
            interfaces.push(wlan0);
        }

        #[cfg(target_os = "macos")]
        {
            let en0 = NetworkInterface::new("en0")
                .with_ipv4("192.168.1.100".parse().unwrap())
                .with_ipv6("fe80::1".parse().unwrap())
                .with_status(true, true);
            interfaces.push(en0);

            let en1 = NetworkInterface::new("en1")
                .with_ipv4("192.168.1.101".parse().unwrap())
                .with_ipv6("fe80::2".parse().unwrap())
                .with_status(true, true);
            interfaces.push(en1);
        }

        debug!("Found {} network interfaces", interfaces.len());
        Ok(interfaces)
    }

    /// Get interfaces that support multicast
    pub fn get_multicast_interfaces() -> Result<Vec<NetworkInterface>> {
        let all_interfaces = get_network_interfaces()?;
        let multicast_interfaces: Vec<NetworkInterface> = all_interfaces
            .into_iter()
            .filter(|iface| iface.is_up && iface.supports_multicast)
            .collect();

        debug!("Found {} multicast-capable interfaces", multicast_interfaces.len());
        Ok(multicast_interfaces)
    }

    /// Check if an IP address is in a private range
    pub fn is_private_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                // 10.0.0.0/8
                octets[0] == 10 ||
                // 172.16.0.0/12
                (octets[0] == 172 && (octets[1] >= 16 && octets[1] <= 31)) ||
                // 192.168.0.0/16
                (octets[0] == 192 && octets[1] == 168) ||
                // 127.0.0.0/8 (loopback)
                octets[0] == 127
            }
            IpAddr::V6(ipv6) => {
                // fe80::/10 (link-local)
                let segments = ipv6.segments();
                (segments[0] & 0xffc0) == 0xfe80 ||
                // ::1 (loopback)
                *ipv6 == Ipv6Addr::LOCALHOST ||
                // fc00::/7 (unique local)
                (segments[0] & 0xfe00) == 0xfc00
            }
        }
    }

    /// Check if an IP address is a loopback address
    pub fn is_loopback_ip(ip: &IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => ipv4.is_loopback(),
            IpAddr::V6(ipv6) => ipv6.is_loopback(),
        }
    }

    /// Get the local IP addresses for a given interface
    pub fn get_interface_addresses(interface_name: &str) -> Result<Vec<IpAddr>> {
        let interfaces = get_network_interfaces()?;
        
        for interface in interfaces {
            if interface.name == interface_name {
                return Ok(interface.all_addresses());
            }
        }

        Err(DiscoveryError::other(format!(
            "Interface '{interface_name}' not found"
        )))
    }

    /// Check if a port is likely to be available for binding
    pub async fn is_port_available(port: u16) -> bool {
        use tokio::net::TcpListener;
        
        (TcpListener::bind(("127.0.0.1", port)).await).is_ok()
    }

    /// Find an available port in a given range
    pub async fn find_available_port(start_port: u16, end_port: u16) -> Option<u16> {
        for port in start_port..=end_port {
            if is_port_available(port).await {
                return Some(port);
            }
        }
        None
    }
}

/// Time utility functions
pub mod time {
    use super::*;

    /// Get current timestamp as seconds since Unix epoch
    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs()
    }

    /// Get current timestamp as milliseconds since Unix epoch
    pub fn current_timestamp_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64
    }

    /// Convert duration to human-readable string
    pub fn duration_to_string(duration: Duration) -> String {
        let total_secs = duration.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;
        let millis = duration.subsec_millis();

        if hours > 0 {
            format!("{hours}h {minutes}m {seconds}s")
        } else if minutes > 0 {
            format!("{minutes}m {seconds}s")
        } else if seconds > 0 {
            format!("{seconds}.{millis:03}s")
        } else {
            format!("{}ms", duration.as_millis())
        }
    }

    /// Check if a duration has elapsed since a given time
    pub fn has_elapsed(since: SystemTime, duration: Duration) -> bool {
        since.elapsed().unwrap_or(Duration::ZERO) >= duration
    }
}

/// String utility functions
pub mod string {
    use super::*;
    use std::collections::HashMap;

    /// Sanitize a service name for use in network protocols
    pub fn sanitize_service_name(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// Validate a service type string
    pub fn validate_service_type(service_type: &str) -> Result<()> {
        if service_type.is_empty() {
            return Err(DiscoveryError::invalid_service("Service type cannot be empty"));
        }

        if !service_type.starts_with('_') {
            return Err(DiscoveryError::invalid_service(
                "Service type must start with underscore",
            ));
        }

        if !service_type.contains("._tcp") && !service_type.contains("._udp") {
            return Err(DiscoveryError::invalid_service(
                "Service type must contain ._tcp or ._udp",
            ));
        }

        Ok(())
    }

    /// Parse key-value pairs from a string (e.g., TXT record format)
    pub fn parse_txt_record(txt_data: &str) -> HashMap<String, String> {
        let mut attributes = HashMap::new();

        for pair in txt_data.split(';') {
            if let Some(eq_pos) = pair.find('=') {
                let key = pair[..eq_pos].trim().to_string();
                let value = pair[eq_pos + 1..].trim().to_string();
                attributes.insert(key, value);
            } else {
                // Key without value
                attributes.insert(pair.trim().to_string(), String::new());
            }
        }

        attributes
    }

    /// Format key-value pairs as TXT record string
    pub fn format_txt_record(attributes: &HashMap<String, String>) -> String {
        attributes
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    k.clone()
                } else {
                    format!("{k}={v}")
                }
            })
            .collect::<Vec<_>>()
            .join(";")
    }
}

/// Validation utility functions
pub mod validation {
    use super::*;

    /// Validate that a port number is in a valid range
    pub fn validate_port(port: u16) -> Result<()> {
        if port == 0 {
            return Err(DiscoveryError::invalid_service("Port cannot be zero"));
        }
        Ok(())
    }

    /// Validate that a timeout is reasonable
    pub fn validate_timeout(timeout: Duration) -> Result<()> {
        if timeout.is_zero() {
            return Err(DiscoveryError::invalid_service("Timeout cannot be zero"));
        }

        if timeout > Duration::from_secs(300) {
            warn!("Timeout is very long: {:?}", timeout);
        }

        Ok(())
    }

    /// Validate an IP address for service discovery
    pub fn validate_ip_address(ip: &IpAddr) -> Result<()> {
        match ip {
            IpAddr::V4(ipv4) => {
                if ipv4.is_unspecified() {
                    return Err(DiscoveryError::invalid_service(
                        "IPv4 address cannot be unspecified (0.0.0.0)",
                    ));
                }
                if ipv4.is_broadcast() {
                    return Err(DiscoveryError::invalid_service(
                        "IPv4 address cannot be broadcast (255.255.255.255)",
                    ));
                }
            }
            IpAddr::V6(ipv6) => {
                if ipv6.is_unspecified() {
                    return Err(DiscoveryError::invalid_service(
                        "IPv6 address cannot be unspecified (::)",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_network_interfaces() {
        let result = network::get_network_interfaces();
        assert!(result.is_ok());
        let interfaces = result.unwrap();
        assert!(!interfaces.is_empty());
        
        // Should always have at least loopback
        assert!(interfaces.iter().any(|i| i.name == "lo"));
    }

    #[test]
    fn test_is_private_ip() {
        assert!(network::is_private_ip(&"192.168.1.1".parse().unwrap()));
        assert!(network::is_private_ip(&"10.0.0.1".parse().unwrap()));
        assert!(network::is_private_ip(&"172.16.0.1".parse().unwrap()));
        assert!(network::is_private_ip(&"127.0.0.1".parse().unwrap()));
        assert!(!network::is_private_ip(&"8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn test_current_timestamp() {
        let timestamp = time::current_timestamp();
        assert!(timestamp > 0);
        
        let timestamp_millis = time::current_timestamp_millis();
        assert!(timestamp_millis > timestamp * 1000);
    }

    #[test]
    fn test_duration_to_string() {
        assert_eq!(time::duration_to_string(Duration::from_millis(500)), "500ms");
        assert_eq!(time::duration_to_string(Duration::from_secs(5)), "5.000s");
        assert_eq!(time::duration_to_string(Duration::from_secs(65)), "1m 5s");
        assert_eq!(time::duration_to_string(Duration::from_secs(3665)), "1h 1m 5s");
    }

    #[test]
    fn test_sanitize_service_name() {
        assert_eq!(string::sanitize_service_name("My Service!"), "My_Service_");
        assert_eq!(string::sanitize_service_name("test-service_1.0"), "test-service_1.0");
    }

    #[test]
    fn test_validate_service_type() {
        assert!(string::validate_service_type("_http._tcp").is_ok());
        assert!(string::validate_service_type("_myservice._udp").is_ok());
        assert!(string::validate_service_type("http._tcp").is_err());
        assert!(string::validate_service_type("_http").is_err());
        assert!(string::validate_service_type("").is_err());
    }

    #[test]
    fn test_parse_txt_record() {
        let txt = "version=1.0;protocol=HTTP;enabled";
        let attrs = string::parse_txt_record(txt);
        
        assert_eq!(attrs.get("version"), Some(&"1.0".to_string()));
        assert_eq!(attrs.get("protocol"), Some(&"HTTP".to_string()));
        assert_eq!(attrs.get("enabled"), Some(&"".to_string()));
    }

    #[test]
    fn test_format_txt_record() {
        let mut attrs = std::collections::HashMap::new();
        attrs.insert("version".to_string(), "1.0".to_string());
        attrs.insert("enabled".to_string(), "".to_string());
        
        let txt = string::format_txt_record(&attrs);
        assert!(txt.contains("version=1.0"));
        assert!(txt.contains("enabled"));
    }

    #[tokio::test]
    async fn test_port_availability() {
        // Test with a likely available port
        let available = network::is_port_available(0).await; // Port 0 should bind to any available port
        assert!(available);
    }

    #[test]
    fn test_validate_port() {
        assert!(validation::validate_port(8080).is_ok());
        assert!(validation::validate_port(0).is_err());
    }

    #[test]
    fn test_validate_timeout() {
        assert!(validation::validate_timeout(Duration::from_secs(5)).is_ok());
        assert!(validation::validate_timeout(Duration::ZERO).is_err());
    }

    #[test]
    fn test_validate_ip_address() {
        assert!(validation::validate_ip_address(&"192.168.1.1".parse().unwrap()).is_ok());
        assert!(validation::validate_ip_address(&"0.0.0.0".parse().unwrap()).is_err());
        assert!(validation::validate_ip_address(&"255.255.255.255".parse().unwrap()).is_err());
        assert!(validation::validate_ip_address(&"::1".parse().unwrap()).is_ok());
        assert!(validation::validate_ip_address(&"::".parse().unwrap()).is_err());
    }
}
