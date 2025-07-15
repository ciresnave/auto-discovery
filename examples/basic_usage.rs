//! Basic usage example for the auto-discovery library
//! 
//! This example demonstrates the core functionality including:
//! - Service registration
//! - Service discovery
//! - Multiple protocols
//! - Proper error handling

use auto_discovery::{
    config::DiscoveryConfig,
    service::ServiceInfo,
    types::{ServiceType, ProtocolType},
    ServiceDiscovery,
};
use std::time::Duration;
use std::net::{IpAddr, Ipv4Addr};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Auto Discovery basic usage example");

    // Configure discovery with multiple service types and protocols
    let config = DiscoveryConfig::new()
        .with_service_type(ServiceType::new("_http._tcp")?)
        .with_service_type(ServiceType::new("_ssh._tcp")?)
        .with_protocol(ProtocolType::Mdns)
        .with_protocol(ProtocolType::Upnp)
        .with_timeout(Duration::from_secs(5))
        .with_verify_services(true);

    info!("📋 Configuration created with mDNS and UPnP protocols");

    // Create discovery instance
    let discovery = ServiceDiscovery::new(config).await?;
    info!("🔍 Service discovery instance created");

    // Register our own HTTP service
    let http_service = ServiceInfo::new(
        "Example Web Server",
        "_http._tcp",
        8080,
        Some(vec![
            ("version", "1.0"),
            ("path", "/api"),
            ("protocol", "HTTP/1.1"),
            ("health", "/health"),
        ])
    )?
    .with_address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

    info!("📝 Registering HTTP service: {}", http_service.name());
    discovery.register_service(http_service.clone()).await?;
    info!("✅ HTTP service registered successfully!");

    // Register an SSH service
    let ssh_service = ServiceInfo::new(
        "Example SSH Server",
        "_ssh._tcp",
        22,
        Some(vec![
            ("version", "OpenSSH_8.0"),
            ("auth", "publickey"),
        ])
    )?
    .with_address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

    info!("📝 Registering SSH service: {}", ssh_service.name());
    discovery.register_service(ssh_service.clone()).await?;
    info!("✅ SSH service registered successfully!");

    // Give services time to propagate
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Start discovery to find services on the network
    info!("🔍 Starting service discovery...");
    let discovered_services = discovery.discover_services(None).await?;
    
    info!("📊 Found {} services:", discovered_services.len());
    for (i, service) in discovered_services.iter().enumerate() {
        info!("  {}. {} ({})", i + 1, service.name(), service.service_type);
        info!("     Address: {}:{}", service.address, service.port);
        info!("     Protocol: {}", service.protocol_type);
        
        if !service.attributes.is_empty() {
            info!("     Attributes:");
            for (key, value) in &service.attributes {
                info!("       {}: {}", key, value);
            }
        }
        info!("");
    }

    // Demonstrate protocol filtering - discover only mDNS services
    info!("🔎 Discovering only mDNS services...");
    let mdns_services = discovery.discover_services(Some(ProtocolType::Mdns)).await?;
    
    info!("📊 Found {} mDNS services:", mdns_services.len());
    for service in &mdns_services {
        info!("  - {} at {}:{}", service.name(), service.address, service.port);
    }

    // Cleanup - unregister our services
    info!("🧹 Cleaning up registered services...");
    discovery.unregister_service(&http_service).await?;
    discovery.unregister_service(&ssh_service).await?;
    info!("✅ Services unregistered successfully");

    info!("🎉 Basic usage example completed successfully!");
    Ok(())
}