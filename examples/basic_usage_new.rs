//! New basic usage example showcasing modern API
//! 
//! This example demonstrates the latest API features and best practices.

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

    info!("Starting new basic usage example");

    // Create configuration with modern features
    let config = DiscoveryConfig::new()
        .with_service_type(ServiceType::new("_http._tcp")?)
        .with_service_type(ServiceType::new("_ssh._tcp")?)
        .with_timeout(Duration::from_secs(10))
        .with_verify_services(true)
        .with_protocol(ProtocolType::Mdns)
        .with_protocol(ProtocolType::Upnp)
        .with_metrics(true);

    info!("Configuration created with modern features");

    // Create service discovery
    let discovery = ServiceDiscovery::new(config).await?;

    // Create a modern service with rich metadata
    let service = ServiceInfo::new(
        "Modern Web API",
        "_http._tcp",
        8080,
        Some(vec![
            ("version", "2.0"),
            ("api_version", "v1"),
            ("health_check", "/health"),
            ("docs", "/docs"),
            ("cors", "enabled"),
        ])
    )?;

    info!("Created service with rich metadata: {}", service.name());

    // Register the service
    discovery.register_service(service.clone()).await?;
    info!("Service registered with modern features");

    // Discover services with protocol filtering
    info!("Discovering services via mDNS...");
    let mdns_services = discovery.discover_services(Some(ProtocolType::Mdns)).await?;
    info!("Found {} services via mDNS", mdns_services.len());

    info!("Discovering services via UPnP...");
    let upnp_services = discovery.discover_services(Some(ProtocolType::Upnp)).await?;
    info!("Found {} services via UPnP", upnp_services.len());

    // Discover all services
    info!("Discovering all services...");
    let all_services = discovery.discover_services(None).await?;
    
    info!("Found {} total services:", all_services.len());
    for service in &all_services {
        info!("  Service: {}", service.name());
        info!("    Address: {}:{}", service.address(), service.port());
        info!("    Protocol: {:?}", service.protocol_type());
        
        if !service.attributes.is_empty() {
            info!("    Attributes:");
            for (key, value) in &service.attributes {
                info!("      {}: {}", key, value);
            }
        }
    }

    // Verify our service
    info!("Verifying our registered service...");
    let is_verified = discovery.verify_service(&service).await?;
    if is_verified {
        info!("✅ Service verification successful");
    } else {
        info!("❌ Service verification failed");
    }

    // Cleanup
    discovery.unregister_service(&service).await?;
    info!("Service unregistered");

    Ok(())
}
