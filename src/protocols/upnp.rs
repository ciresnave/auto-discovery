//! UPnP (Universal Plug and Play) and SSDP protocol implementation with real multicast support

use crate::{
    config::DiscoveryConfig,
    error::Result,
    registry::ServiceRegistry,
    service::ServiceInfo,
    types::{ServiceType, ProtocolType},
    protocols::DiscoveryProtocol,
};
use async_trait::async_trait;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    net::UdpSocket,
    sync::{oneshot, RwLock},
    task::JoinHandle,
};
use tracing::{debug, error, info};

/// SSDP (Simple Service Discovery Protocol) implementation for UPnP discovery
pub struct SsdpProtocol {
    registry: Arc<ServiceRegistry>,
    #[allow(dead_code)]
    config: DiscoveryConfig,
    /// Background listener task handle
    listener_handle: Option<JoinHandle<()>>,
    /// Shutdown channel sender
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Registered services for responding to search requests
    registered_services: Arc<RwLock<HashMap<String, ServiceInfo>>>,
}

impl SsdpProtocol {
    /// Create a new SSDP protocol instance
    pub fn new(config: DiscoveryConfig) -> Result<Self> {
        let registry = Arc::new(ServiceRegistry::new());
        let registered_services = Arc::new(RwLock::new(HashMap::new()));

        Ok(Self {
            registry,
            config,
            listener_handle: None,
            shutdown_tx: None,
            registered_services,
        })
    }

    /// Start the SSDP listener
    pub async fn start_listener(&mut self) -> Result<()> {
        if self.listener_handle.is_some() {
            return Ok(());
        }

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let registered_services = self.registered_services.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = Self::run_listener(registered_services, shutdown_rx).await {
                error!("SSDP listener error: {}", e);
            }
        });

        self.listener_handle = Some(handle);
        info!("SSDP listener started");

        Ok(())
    }

    /// Start the SSDP listener in the background
    async fn run_listener(
        registered_services: Arc<RwLock<HashMap<String, ServiceInfo>>>,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:1900").await?;
        socket.set_broadcast(true)?;
        
        socket.join_multicast_v4("239.255.255.250".parse().unwrap(), "0.0.0.0".parse().unwrap())?;
        
        let mut buf = [0u8; 1024];
        
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => break,
                result = socket.recv_from(&mut buf) => {
                    match result {
                        Ok((len, addr)) => {
                            let message = String::from_utf8_lossy(&buf[..len]);
                            if message.contains("M-SEARCH") {
                                // Handle M-SEARCH request
                                let search_target = Self::parse_search_target(&message);
                                let services = registered_services.read().await;
                                for service in services.values() {
                                    if Self::service_matches_search(&search_target, service) {
                                        let _ = Self::send_response(&socket, addr, service).await;
                                    }
                                }
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Parse search target from M-SEARCH message
    fn parse_search_target(message: &str) -> String {
        for line in message.lines() {
            if let Some(stripped) = line.strip_prefix("ST:") {
                return stripped.trim().to_string();
            }
        }
        "ssdp:all".to_string()
    }

    /// Check if a service matches the search target
    fn service_matches_search(search_target: &str, service: &ServiceInfo) -> bool {
        match search_target {
            "ssdp:all" | "upnp:rootdevice" => true,
            target => {
                // Check if the search target matches the service type
                target == service.service_type.to_string() || 
                service.service_type.to_string().contains(target)
            }
        }
    }

    /// Send a response to an M-SEARCH request
    async fn send_response(socket: &UdpSocket, addr: SocketAddr, service: &ServiceInfo) -> Result<()> {
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
            CACHE-CONTROL: max-age=1800\r\n\
            DATE: {}\r\n\
            EXT:\r\n\
            LOCATION: http://{}:{}/\r\n\
            SERVER: AutoDiscovery/1.0 UPnP/1.0\r\n\
            ST: upnp:rootdevice\r\n\
            USN: uuid:{}::upnp:rootdevice\r\n\
            \r\n",
            chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT"),
            service.address,
            service.port,
            service.id
        );
        
        socket.send_to(response.as_bytes(), addr).await?;
        Ok(())
    }

    /// Send an SSDP search request
    async fn send_search_request(service_type: &str, timeout_secs: u64) -> Result<UdpSocket> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;
        
        let search_msg = format!(
            "M-SEARCH * HTTP/1.1\r\n\
            HOST: 239.255.255.250:1900\r\n\
            MAN: \"ssdp:discover\"\r\n\
            ST: {service_type}\r\n\
            MX: {timeout_secs}\r\n\
            \r\n"
        );
        
        let multicast_addr: SocketAddr = "239.255.255.250:1900".parse().unwrap();
        socket.send_to(search_msg.as_bytes(), multicast_addr).await?;
        
        Ok(socket)
    }

    /// Send an SSDP announcement
    async fn send_announcement(service: &ServiceInfo, notification_type: &str) -> Result<()> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.set_broadcast(true)?;
        
        let announcement = format!(
            "NOTIFY * HTTP/1.1\r\n\
            HOST: 239.255.255.250:1900\r\n\
            CACHE-CONTROL: max-age=1800\r\n\
            LOCATION: http://{}:{}/\r\n\
            NT: upnp:rootdevice\r\n\
            NTS: {}\r\n\
            USN: uuid:{}::upnp:rootdevice\r\n\
            SERVER: AutoDiscovery/1.0 UPnP/1.0\r\n\
            \r\n",
            service.address,
            service.port,
            notification_type,
            service.id
        );
        
        let multicast_addr: SocketAddr = "239.255.255.250:1900".parse().unwrap();
        socket.send_to(announcement.as_bytes(), multicast_addr).await?;
        
        Ok(())
    }

    /// Parse service information from SSDP response
    fn parse_service_from_response(response: &str, addr: SocketAddr) -> Option<ServiceInfo> {
        let mut location = None;
        let mut usn = None;
        
        for line in response.lines() {
            if let Some(stripped) = line.strip_prefix("LOCATION:") {
                location = Some(stripped.trim().to_string());
            } else if let Some(stripped) = line.strip_prefix("USN:") {
                usn = Some(stripped.trim().to_string());
            }
        }
        
        if let (Some(location), Some(usn)) = (location, usn) {
            let service_id = usn.split("::").next().unwrap_or("unknown").to_string();
            let mut service = ServiceInfo::new(
                service_id,
                "upnp._tcp",
                addr.port(),
                Some(vec![
                    ("location", &location),
                    ("usn", &usn),
                ])
            ).ok()?;
            
            service.address = addr.ip();
            
            Some(service)
        } else {
            None
        }
    }
}

#[async_trait]
impl DiscoveryProtocol for SsdpProtocol {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::Upnp
    }

    /// Discover services of the specified types with timeout
    async fn discover_services(
        &self,
        service_types: Vec<ServiceType>,
        timeout: Option<Duration>,
    ) -> Result<Vec<ServiceInfo>> {
        let mut services = Vec::new();
        let timeout_duration = timeout.unwrap_or(Duration::from_secs(10)).min(Duration::from_secs(30));
        let start_time = Instant::now();

        debug!("Starting UPnP discovery for service types: {:?}", service_types);

        // Send search request for each service type
        for service_type in service_types {
            let socket = Self::send_search_request(&service_type.to_string(), timeout_duration.as_secs()).await?;

            let mut buf = [0u8; 2048];
            while start_time.elapsed() < timeout_duration {
                let remaining = timeout_duration - start_time.elapsed();
                if remaining.is_zero() {
                    break;
                }

                match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
                    Ok(Ok((len, addr))) => {
                        let response = String::from_utf8_lossy(&buf[..len]);
                        if let Some(service) = Self::parse_service_from_response(&response, addr) {
                            debug!("Discovered UPnP service: {:?}", service);
                            services.push(service);
                        }
                    }
                    Ok(Err(_)) => break,
                    Err(_) => break,
                }
            }
        }

        info!("UPnP discovery found {} services", services.len());
        Ok(services)
    }

    async fn register_service(&self, service: ServiceInfo) -> Result<()> {
        // Store in our registered services for responding to searches
        let mut services = self.registered_services.write().await;
        services.insert(service.id.to_string(), service.clone());

        // Send announcement
        Self::send_announcement(&service, "ssdp:alive").await?;

        info!("Registered UPnP service: {} ({}:{})", service.name, service.address, service.port);
        Ok(())
    }

    async fn unregister_service(&self, service: &ServiceInfo) -> Result<()> {
        let service_id = service.id.to_string();
        
        // Remove from our registered services
        let mut services = self.registered_services.write().await;
        if let Some(service) = services.remove(&service_id) {
            // Send byebye announcement
            Self::send_announcement(&service, "ssdp:byebye").await?;
            info!("Unregistered UPnP service: {} ({}:{})", service.name, service.address, service.port);
        }

        Ok(())
    }

    async fn verify_service(&self, service: &ServiceInfo) -> Result<bool> {
        // For UPnP, check if the service is in our registered services
        let services = self.registered_services.read().await;
        let is_registered = services.contains_key(&service.id.to_string());
        
        if is_registered {
            debug!("UPnP service verified: {} ({}:{})", service.name, service.address, service.port);
            Ok(true)
        } else {
            debug!("UPnP service not found in registered services: {} ({}:{})", service.name, service.address, service.port);
            Ok(false)
        }
    }

    async fn is_available(&self) -> bool {
        // UPnP/SSDP is generally available on most networks
        true
    }

    fn set_registry(&mut self, registry: Arc<ServiceRegistry>) {
        self.registry = registry;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_ssdp_protocol_creation() {
        let config = DiscoveryConfig::new();
        let protocol = SsdpProtocol::new(config);
        assert!(protocol.is_ok());
    }

    #[tokio::test]
    async fn test_search_target_parsing() {
        let message = "M-SEARCH * HTTP/1.1\r\nST: upnp:rootdevice\r\n\r\n";
        let target = SsdpProtocol::parse_search_target(message);
        assert_eq!(target, "upnp:rootdevice");
    }

    #[tokio::test]
    async fn test_service_matching() {
        let service = ServiceInfo::new(
            "test-service",
            "upnp._tcp",
            8080,
            None
        ).unwrap();
        
        assert!(SsdpProtocol::service_matches_search("ssdp:all", &service));
        assert!(SsdpProtocol::service_matches_search("upnp:rootdevice", &service));
        assert!(!SsdpProtocol::service_matches_search("specific:service", &service));
    }

    #[tokio::test]
    async fn test_service_registration() {
        let config = DiscoveryConfig::new();
        let protocol = SsdpProtocol::new(config).unwrap();
        
        let service = ServiceInfo::new(
            "test-service",
            "upnp._tcp",
            8080,
            None
        ).unwrap();
        
        let result = protocol.register_service(service).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_service_discovery() {
        let config = DiscoveryConfig::new();
        let protocol = SsdpProtocol::new(config).unwrap();
        
        let service_type = ServiceType::new("upnp._tcp").unwrap();
        let service_types = vec![service_type];
        let timeout = Some(Duration::from_secs(1));
        
        let result = protocol.discover_services(service_types, timeout).await;
        assert!(result.is_ok());
    }
}
