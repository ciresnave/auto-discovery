//! Simple discovery example (fixed version)
//! 
//! This is a minimal, working example of service discovery.

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

    info!("Simple discovery example (fixed)");

    // Simple configuration
    let config = DiscoveryConfig::new()
        .with_service_type(ServiceType::new("_http._tcp")?)
        .with_timeout(Duration::from_secs(5));

    // Create discovery
    let discovery = ServiceDiscovery::new(config).await?;

    // Create and register a simple service
    let service = ServiceInfo::new(
        "Simple Test Service",
        "_http._tcp",
        8080,
        None
    )?;

    discovery.register_service(service.clone()).await?;
    info!("Service registered");

    // Discover services
    let services = discovery.discover_services(None).await?;
    info!("Found {} services", services.len());

    for service in services {
        info!("  {}", service.name());
    }

    // Cleanup
    discovery.unregister_service(&service).await?;
    info!("Done");

    Ok(())
}
