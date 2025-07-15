//! DNS Service Discovery example
//! 
//! This example demonstrates DNS-SD protocol usage for service discovery.

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

    info!("Starting DNS-SD example");

    // Configure DNS-SD discovery
    let config = DiscoveryConfig::new()
        .with_service_type(ServiceType::new("_http._tcp.local")?)
        .with_service_type(ServiceType::new("_ssh._tcp.local")?)
        .with_protocol(ProtocolType::DnsSd)
        .with_timeout(Duration::from_secs(5));

    info!("Creating DNS-SD discovery service");
    let discovery = ServiceDiscovery::new(config).await?;

    // Register a DNS-SD service
    let service = ServiceInfo::new(
        "DNS-SD Web Service",
        "_http._tcp.local",
        8080,
        Some(vec![
            ("version", "1.0"),
            ("protocol", "DNS-SD"),
            ("path", "/api"),
        ])
    )?;

    info!("Registering service: {}", service.name());
    discovery.register_service(service.clone()).await?;

    // Discover DNS-SD services
    info!("Discovering DNS-SD services...");
    let discovered = discovery.discover_services(None).await?;
    
    info!("Found {} DNS-SD services:", discovered.len());
    for service in &discovered {
        info!("  - {} at {}:{}", service.name(), service.address, service.port);
        for (key, value) in &service.attributes {
            info!("    {}: {}", key, value);
        }
    }

    // Cleanup
    discovery.unregister_service(&service).await?;
    info!("DNS-SD example completed");

    Ok(())
}