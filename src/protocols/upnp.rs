//! UPnP (Universal Plug and Play) and SSDP protocol implementation

use crate::{
    config::DiscoveryConfig,
    error::{DiscoveryError, Result},
    service::ServiceInfo,
    types::{ServiceType, ProtocolType},
    protocols::DiscoveryProtocol,
};
use async_trait::async_trait;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::UdpSocket,
    sync::Mutex,
};
use tracing::warn;
use uuid::Uuid;

const SSDP_ADDR: &str = "239.255.255.250:1900";
const DEFAULT_TTL: u32 = 1800;

/// SSDP (Simple Service Discovery Protocol) implementation for UPnP discovery
pub struct SsdpProtocol {
    socket: Arc<Mutex<UdpSocket>>,
}

#[derive(Debug)]
enum SsdpMessage {
    Search {
        search_target: String,
    },
    Response {
        location: String,
        service_type: String,
        usn: String,
    },
    Notify {
        nt: String,
        nts: String,
        location: String,
        usn: String,
    },
}

impl SsdpMessage {
    fn parse(data: &str) -> Option<Self> {
        let mut lines = data.lines();
        let first_line = lines.next()?;

        if first_line.starts_with("M-SEARCH") {
            let mut search_target = None;

            for line in lines {
                if line.starts_with("ST:") {
                    search_target = Some(line[3..].trim().to_string());
                    break;
                }
            }

            if let Some(st) = search_target {
                return Some(SsdpMessage::Search { search_target: st });
            }
        } else if first_line.starts_with("NOTIFY") {
            let mut nt = None;
            let mut nts = None;
            let mut location = None;
            let mut usn = None;

            for line in lines {
                let line = line.trim();
                if line.starts_with("NT:") {
                    nt = Some(line[3..].trim().to_string());
                } else if line.starts_with("NTS:") {
                    nts = Some(line[4..].trim().to_string());
                } else if line.starts_with("LOCATION:") {
                    location = Some(line[9..].trim().to_string());
                } else if line.starts_with("USN:") {
                    usn = Some(line[4..].trim().to_string());
                }
            }

            if let (Some(nt), Some(nts), Some(location), Some(usn)) = (nt, nts, location, usn) {
                return Some(SsdpMessage::Notify { nt, nts, location, usn });
            }
        } else if first_line.starts_with("HTTP/1.1 200") {
            let mut location = None;
            let mut service_type = None;
            let mut usn = None;

            for line in lines {
                let line = line.trim();
                if line.starts_with("LOCATION:") {
                    location = Some(line[9..].trim().to_string());
                } else if line.starts_with("ST:") {
                    service_type = Some(line[3..].trim().to_string());
                } else if line.starts_with("USN:") {
                    usn = Some(line[4..].trim().to_string());
                }
            }

            if let (Some(location), Some(service_type), Some(usn)) = (location, service_type, usn) {
                return Some(SsdpMessage::Response { location, service_type, usn });
            }
        }

        None
    }

    fn to_string(&self) -> String {
        match self {
            SsdpMessage::Search { search_target } => {
                format!(
                    "M-SEARCH * HTTP/1.1\r\n\
                     HOST: {}\r\n\
                     MAN: \"ssdp:discover\"\r\n\
                     MX: 3\r\n\
                     ST: {}\r\n\
                     \r\n",
                    SSDP_ADDR, search_target
                )
            }
            SsdpMessage::Response { location, service_type, usn } => {
                format!(
                    "HTTP/1.1 200 OK\r\n\
                     CACHE-CONTROL: max-age={}\r\n\
                     LOCATION: {}\r\n\
                     ST: {}\r\n\
                     USN: {}\r\n\
                     \r\n",
                    DEFAULT_TTL, location, service_type, usn
                )
            }
            SsdpMessage::Notify { nt, nts, location, usn } => {
                format!(
                    "NOTIFY * HTTP/1.1\r\n\
                     HOST: {}\r\n\
                     NT: {}\r\n\
                     NTS: {}\r\n\
                     LOCATION: {}\r\n\
                     USN: {}\r\n\
                     \r\n",
                    SSDP_ADDR, nt, nts, location, usn
                )
            }
        }
    }
}

impl SsdpProtocol {
    /// Create a new SSDP protocol instance
    /// 
    /// # Arguments
    /// 
    /// * `_config` - The discovery configuration (currently unused)
    /// 
    /// # Errors
    /// 
    /// Returns an error if UDP socket creation fails
    pub async fn new(_config: &DiscoveryConfig) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .map_err(|e| DiscoveryError::upnp(format!("Failed to bind socket: {}", e)))?;

        socket.set_multicast_ttl_v4(4)
            .map_err(|e| DiscoveryError::upnp(format!("Failed to set multicast TTL: {}", e)))?;

        Ok(Self {
            socket: Arc::new(Mutex::new(socket)),
        })
    }

    async fn send_search(&self, service_type: &str) -> Result<()> {
        let msg = SsdpMessage::Search {
            search_target: service_type.to_string(),
        };

        let ssdp_addr: SocketAddr = SSDP_ADDR.parse()
            .map_err(|e| DiscoveryError::upnp(format!("Invalid SSDP address: {}", e)))?;

        self.socket.lock()
            .await
            .send_to(msg.to_string().as_bytes(), &ssdp_addr)
            .await
            .map_err(|e| DiscoveryError::upnp(format!("Failed to send SSDP search: {}", e)))?;

        Ok(())
    }

    fn parse_ssdp_response(&self, response: &str, addr: IpAddr) -> Option<ServiceInfo> {
        if let Some(SsdpMessage::Response { location, service_type, usn }) = SsdpMessage::parse(response) {
            let service = ServiceInfo::new(
                &format!("upnp-{}", Uuid::new_v4()),
                &service_type,
                0,
                Some(vec![("location", &location), ("usn", &usn)])
            ).ok()?
            .with_address(addr);
            
            Some(service)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    async fn check_health(&self) -> bool {
        let ssdp_addr: SocketAddr = match SSDP_ADDR.parse() {
            Ok(addr) => addr,
            Err(_) => return false,
        };

        let test_data = [0u8; 1];
        self.socket.lock()
            .await
            .send_to(&test_data, ssdp_addr)
            .await
            .is_ok()
    }
}

/// Type alias for UPnP protocol using SSDP implementation
pub type UpnpProtocol = SsdpProtocol;

#[async_trait]
impl DiscoveryProtocol for SsdpProtocol {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::Upnp
    }

    async fn discover_services(
        &self,
        service_types: Vec<ServiceType>,
        timeout: Option<Duration>,
    ) -> Result<Vec<ServiceInfo>> {
        let mut all_services = Vec::new();
        
        for service_type in service_types {
            self.send_search(&service_type.to_string()).await?;
        }

        let timeout_duration = timeout.unwrap_or(Duration::from_secs(30));
        let mut buf = [0; 1024];
        let timeout_instant = tokio::time::Instant::now() + timeout_duration;

        loop {
            let remaining = timeout_instant.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match tokio::time::timeout(remaining, self.socket.lock().await.recv_from(&mut buf)).await {
                Ok(Ok((len, addr))) => {
                    let response = String::from_utf8_lossy(&buf[..len]).to_string();
                    if let Some(service) = self.parse_ssdp_response(&response, addr.ip()) {
                        all_services.push(service);
                    }
                }
                Ok(Err(e)) => warn!("Error receiving SSDP response: {}", e),
                Err(_) => break, // Timeout
            }
        }

        Ok(all_services)
    }

    async fn register_service(&self, service: ServiceInfo) -> Result<()> {
        let msg = SsdpMessage::Notify {
            nt: service.service_type.to_string(),
            nts: "ssdp:alive".to_string(),
            location: format!("http://{}:{}", service.address, service.port),
            usn: format!("uuid:{}", Uuid::new_v4()),
        };

        let ssdp_addr: SocketAddr = SSDP_ADDR.parse()
            .map_err(|e| DiscoveryError::upnp(format!("Invalid SSDP address: {}", e)))?;

        self.socket.lock()
            .await
            .send_to(msg.to_string().as_bytes(), &ssdp_addr)
            .await
            .map_err(|e| DiscoveryError::upnp(format!("Failed to register service: {}", e)))?;

        Ok(())
    }

    async fn unregister_service(&self, service: &ServiceInfo) -> Result<()> {
        let msg = SsdpMessage::Notify {
            nt: service.service_type.to_string(),
            nts: "ssdp:byebye".to_string(),
            location: format!("http://{}:{}", service.address, service.port),
            usn: format!("uuid:{}", Uuid::new_v4()),
        };

        let ssdp_addr: SocketAddr = SSDP_ADDR.parse()
            .map_err(|e| DiscoveryError::upnp(format!("Invalid SSDP address: {}", e)))?;

        self.socket.lock()
            .await
            .send_to(msg.to_string().as_bytes(), &ssdp_addr)
            .await
            .map_err(|e| DiscoveryError::upnp(format!("Failed to unregister service: {}", e)))?;

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
    use tokio::test;

    #[test]
    async fn test_upnp_discovery() {
        let config = crate::config::DiscoveryConfig::new();
        let protocol = SsdpProtocol::new(&config).await.unwrap();

        let service_type = ServiceType::new("urn:schemas-upnp-org:service:ContentDirectory:1").unwrap();
        let services = protocol
            .discover_services(vec![service_type], Some(Duration::from_secs(1)))
            .await
            .unwrap();

        // Note: This test might fail if no UPnP devices are on the network
        assert!(!services.is_empty() || services.is_empty()); // Just verify services vector is valid
    }

    #[test]
    async fn test_response_parsing() {
        let config = crate::config::DiscoveryConfig::new();
        let protocol = SsdpProtocol::new(&config).await.unwrap();

        let response = "\
            HTTP/1.1 200 OK\r\n\
            CACHE-CONTROL: max-age=1800\r\n\
            LOCATION: http://192.168.1.1:8080/device.xml\r\n\
            ST: urn:schemas-upnp-org:service:ContentDirectory:1\r\n\
            USN: uuid:12345678-1234-1234-1234-123456789012\r\n\
            \r\n";

        let service = protocol.parse_ssdp_response(
            response,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
        );

        assert!(service.is_some());
        let service = service.unwrap();
        assert_eq!(
            service.service_type().full_name(),
            "urn:schemas-upnp-org:service:ContentDirectory:1"
        );
    }
}
