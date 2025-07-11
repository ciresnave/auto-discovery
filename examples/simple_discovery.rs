use auto_discovery::{ServiceDiscovery, DiscoveryConfig, ServiceType, ServiceInfo};
use std::net::Ipv4Addr;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting simple discovery example");

    // Create a simple configuration
    let config = DiscoveryConfig::new()
        .with_service_type(ServiceType::new("_example._tcp")?)
        .with_timeout(Duration::from_secs(3));

    // Create discovery instance
    let discovery = ServiceDiscovery::new(config).await?;

    // Create a local service to register
    let service = ServiceInfo::new(
        "Example Service",
        "_example._tcp",
        8080,
        Some(vec![
            ("version", "1.0"),
            ("status", "running")
        ])
    )?
    .with_address(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST));

    info!("Registering local service: {}", service.name());

    // Register the service
    discovery.register_service(service.clone()).await?;

    // Give some time for registration
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Try to discover services
    info!("Discovering services on the network...");
    let discovered = discovery.discover_services(None).await?;

    if discovered.is_empty() {
        info!("No services found");
    } else {
        info!("Found {} service(s):", discovered.len());
        for svc in discovered {
            info!("  - {} at {}:{}", svc.name(), svc.address, svc.port);
        }
    }

    // Clean up
    discovery.unregister_service(&service).await?;
    info!("Service unregistered");

    Ok(())
}
