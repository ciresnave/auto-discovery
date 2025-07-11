//! Example demonstrating the builder pattern for configuration
//! 
//! This example shows how to use the builder pattern to create
//! discovery configurations and protocol managers.

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

    info!("Starting builder pattern example");

    // Use builder pattern for discovery configuration
    let config = DiscoveryConfig::new()
        .with_timeout(Duration::from_secs(30))
        .with_verify_services(true)
        .with_service_type(ServiceType::new("_http._tcp")?)
        .with_protocol(ProtocolType::Mdns)
        .with_protocol(ProtocolType::Upnp);

    info!("Configuration created with builder pattern");

    // Create service discovery instance
    let discovery = ServiceDiscovery::new(config).await?;

    info!("Service discovery created with builder-configured settings");

    // Create a service using builder-like method chaining
    let service = ServiceInfo::new(
        "Builder Example Service",
        "_http._tcp",
        8080,
        Some(vec![
            ("version", "1.0"),
            ("api", "REST"),
            ("build_tool", "builder_pattern"),
        ])
    )?;

    info!("Service created: {}", service.name());

    // Register the service
    discovery.register_service(service.clone()).await?;
    info!("Service registered successfully");

    // Discover services using the configured discovery
    let services = discovery.discover_services(Some(ProtocolType::Mdns)).await?;

    info!("Discovered {} services using builder-configured discovery", services.len());
    for service in services {
        info!("  - {} at {}:{}", service.name(), service.address(), service.port());
    }

    // Cleanup
    discovery.unregister_service(&service).await?;
    info!("Service unregistered");

    Ok(())
}
