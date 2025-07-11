//! Example demonstrating cross-protocol service discovery
//! 
//! This example shows how to discover services across multiple protocols
//! and handle different protocol-specific features.

use auto_discovery::{
    config::DiscoveryConfig,
    service::ServiceInfo,
    types::{ServiceType, ProtocolType},
    ServiceDiscovery,
};
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting cross-protocol discovery example");

    // Configure discovery with multiple protocols
    let config = DiscoveryConfig::new()
        .with_timeout(Duration::from_secs(10))
        .with_verify_services(true)
        .with_service_type(ServiceType::new("_http._tcp")?) // mDNS service type
        .with_service_type(ServiceType::new("_ssh._tcp")?)  // SSH services
        .with_protocol(ProtocolType::Mdns)
        .with_protocol(ProtocolType::Upnp)
        .with_protocol(ProtocolType::DnsSd)
        .with_cross_protocol(true); // Enable cross-protocol discovery

    info!("Configuration created with multiple protocols enabled");

    // Create service discovery instance
    let discovery = ServiceDiscovery::new(config).await?;

    info!("Service discovery created with cross-protocol support");

    // Register a service that can be discovered by multiple protocols
    let service = ServiceInfo::new(
        "Cross-Protocol Web Service",
        "_http._tcp",
        8080,
        Some(vec![
            ("version", "1.0"),
            ("protocol", "multi"),
            ("discovery", "cross-protocol"),
        ])
    )?;

    info!("Registering service: {}", service.name());
    discovery.register_service(service.clone()).await?;
    info!("Service registered successfully");

    // Discover services using mDNS
    info!("Discovering services via mDNS...");
    let mdns_services = discovery.discover_services(Some(ProtocolType::Mdns)).await?;
    info!("Found {} services via mDNS", mdns_services.len());
    for service in &mdns_services {
        info!("  mDNS: {} at {}:{}", service.name(), service.address(), service.port());
    }

    // Discover services using UPnP
    info!("Discovering services via UPnP...");
    let upnp_services = discovery.discover_services(Some(ProtocolType::Upnp)).await?;
    info!("Found {} services via UPnP", upnp_services.len());
    for service in &upnp_services {
        info!("  UPnP: {} at {}:{}", service.name(), service.address(), service.port());
    }

    // Discover services across all protocols
    info!("Discovering services across all protocols...");
    let all_services = discovery.discover_services(None).await?;
    info!("Found {} total services across all protocols", all_services.len());

    // Group services by protocol for analysis
    let mut protocol_counts = std::collections::HashMap::new();
    for service in &all_services {
        let count = protocol_counts.entry(service.protocol_type()).or_insert(0);
        *count += 1;
    }

    info!("Services by protocol:");
    for (protocol, count) in protocol_counts {
        info!("  {:?}: {} services", protocol, count);
    }

    // Find services that are discoverable via multiple protocols
    let mut service_names = std::collections::HashMap::new();
    for service in &all_services {
        let protocols = service_names.entry(service.name().to_string()).or_insert(Vec::new());
        protocols.push(service.protocol_type());
    }

    info!("Services discoverable via multiple protocols:");
    for (name, protocols) in service_names {
        if protocols.len() > 1 {
            info!("  {}: {:?}", name, protocols);
        }
    }

    // Cleanup
    discovery.unregister_service(&service).await?;
    info!("Service unregistered");

    Ok(())
}
