//! Simple discovery example
//! 
//! This is a minimal, beginner-friendly example demonstrating:
//! - Basic service registration
//! - Simple service discovery
//! - Clean resource management

use auto_discovery::{
    config::DiscoveryConfig,
    service::ServiceInfo,
    types::ServiceType,
    ServiceDiscovery,
};
use std::net::Ipv4Addr;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for output
    tracing_subscriber::fmt::init();

    info!("🔍 Simple discovery example starting");

    // Create a minimal configuration
    let config = DiscoveryConfig::new()
        .with_service_type(ServiceType::new("_test._tcp")?)
        .with_timeout(Duration::from_secs(3));

    // Create discovery instance
    let discovery = ServiceDiscovery::new(config).await?;
    info!("✅ Discovery service created");

    // Create a simple test service
    let service = ServiceInfo::new(
        "Simple Test Service",
        "_test._tcp",
        8080,
        Some(vec![
            ("version", "1.0"),
            ("status", "running"),
            ("example", "simple"),
        ])
    )?
    .with_address(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST));

    info!("📝 Registering service: {}", service.name());

    // Register the service
    discovery.register_service(service.clone()).await?;
    info!("✅ Service registered successfully");

    // Brief pause to allow registration to propagate
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Discover services on the network
    info!("🔍 Discovering services...");
    let discovered = discovery.discover_services(None).await?;

    if discovered.is_empty() {
        info!("📭 No services found on the network");
    } else {
        info!("📊 Found {} service(s):", discovered.len());
        for (i, svc) in discovered.iter().enumerate() {
            info!("  {}. {} at {}:{}", i + 1, svc.name(), svc.address, svc.port);
            
            // Show attributes if present
            if !svc.attributes.is_empty() {
                for (key, value) in &svc.attributes {
                    info!("     {}: {}", key, value);
                }
            }
        }
    }

    // Clean up by unregistering our service
    info!("🧹 Unregistering service...");
    discovery.unregister_service(&service).await?;
    info!("✅ Service unregistered");

    info!("🎉 Simple discovery example completed");
    Ok(())
}