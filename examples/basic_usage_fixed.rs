//! Basic usage example with fixed imports and simplified API
//! 
//! This is a streamlined version of the basic usage example.

use auto_discovery::{
    config::DiscoveryConfig,
    service::ServiceInfo,
    types::ServiceType,
    ServiceDiscovery,
};
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting basic usage example (fixed)");

    // Configure discovery
    let config = DiscoveryConfig::new()
        .with_service_type(ServiceType::new("_http._tcp")?)
        .with_timeout(Duration::from_secs(5))
        .with_verify_services(false);

    // Create discovery instance
    let discovery = ServiceDiscovery::new(config).await?;

    // Register our service
    let service = ServiceInfo::new(
        "Example Service (Fixed)",
        "_http._tcp",
        8080,
        Some(vec![
            ("version", "1.0"),
            ("example", "fixed"),
        ])
    )?;

    info!("Registering service: {}", service.name());
    discovery.register_service(service.clone()).await?;
    info!("Service registered successfully!");

    // Discover services
    info!("Discovering services...");
    let discovered_services = discovery.discover_services(None).await?;
    
    info!("Found {} services:", discovered_services.len());
    for service in &discovered_services {
        info!("  - {} at {}:{}", service.name(), service.address(), service.port());
    }

    // Unregister service
    discovery.unregister_service(&service).await?;
    info!("Service unregistered");

    Ok(())
}
