use auto_discovery::{ServiceDiscovery, DiscoveryConfig, ServiceType, ServiceInfo};
use std::time::Duration;
use std::net::{IpAddr, Ipv4Addr};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Auto Discovery example");

    // Configure discovery
    let config = DiscoveryConfig::new()
        .with_service_type(ServiceType::new("_http._tcp")?)
        .with_service_type(ServiceType::new("_ssh._tcp")?)
        .with_timeout(Duration::from_secs(5))
        .with_verify_services(true);

    // Create discovery instance
    let discovery = ServiceDiscovery::new(config).await?;

    // Register our own service
    let service = ServiceInfo::new(
        "Example Web Server",
        "_http._tcp",
        8080,
        Some(vec![
            ("version", "1.0"),
            ("path", "/api"),
            ("protocol", "HTTP/1.1"),
        ])
    )?
    .with_address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

    info!("Registering service: {}", service.name());

    // Register the service
    discovery.register_service(service.clone()).await?;

    info!("Service registered successfully!");

    // Start discovery to find other services
    info!("Starting service discovery...");
    
    // Discover services on the network
    let discovered_services = discovery.discover_services(None).await?;
    
    info!("Found {} services:", discovered_services.len());
    for service in &discovered_services {
        info!("✓ Discovered service: {}", service.name());
        info!("  Type: {}", service.service_type().service_name());
        info!("  Address: {}", service.address);
        info!("  Port: {}", service.port);
        
        if !service.attributes.is_empty() {
            info!("  Attributes:");
            for (key, value) in &service.attributes {
                info!("    {}: {}", key, value);
            }
        }
    }

    if discovered_services.is_empty() {
        info!("No services discovered. This might be because:");
        info!("  - No compatible services are running on the network");
        info!("  - Firewall is blocking service discovery");
        info!("  - Services are using different service types");
    }

    // Verify our registered service
    let verified = discovery.verify_service(&service).await?;
    if verified {
        info!("✓ Our service is verified and accessible");
    } else {
        info!("✗ Our service verification failed");
    }

    // Unregister our service
    discovery.unregister_service(&service).await?;
    info!("Service unregistered");

    Ok(())
}
