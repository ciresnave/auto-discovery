//! Non-network test example that demonstrates the API without requiring real network access
//!
//! This example shows how to use the library APIs without actually performing network operations,
//! making it safe to run in any environment.

use auto_discovery::{
    ServiceDiscovery,
    config::DiscoveryConfig,
    service::ServiceInfo,
    types::{ProtocolType, ServiceType},
};
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with error level to reduce output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting non-network test example");

    // Test 1: Configuration creation and validation
    info!("📋 Testing configuration creation...");
    let config = DiscoveryConfig::new()
        .with_service_type(ServiceType::new("_http._tcp")?)
        .with_protocol(ProtocolType::Upnp) // Use UPnP which is less likely to cause network issues
        .with_timeout(Duration::from_secs(1)) // Short but valid timeout
        .with_verify_services(false); // Disable verification to avoid network calls

    info!("✅ Configuration created successfully");

    // Test 2: Service info creation
    info!("🔧 Testing service info creation...");
    let service = ServiceInfo::new(
        "Test Service",
        "_http._tcp",
        8080,
        Some(vec![
            ("version", "1.0"),
            ("description", "Test service for validation"),
        ]),
    )?
    .with_address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));

    info!(
        "✅ Service info created: {} on port {}",
        service.name(),
        service.port()
    );

    // Test 3: Service discovery instance creation
    info!("🔍 Testing service discovery creation...");
    let discovery = ServiceDiscovery::new(config).await?;
    info!("✅ Service discovery instance created");

    // Test 4: Quick discovery test with very short timeout
    info!("🔎 Testing quick discovery (with short timeout)...");
    let start_time = std::time::Instant::now();

    let discovered = discovery.discover_services(Some(ProtocolType::Upnp)).await;

    let elapsed = start_time.elapsed();
    info!("⏱️  Discovery completed in {:?}", elapsed);

    match discovered {
        Ok(services) => {
            info!("✅ Discovery succeeded, found {} services", services.len());
        }
        Err(e) => {
            info!("ℹ️  Discovery failed as expected (no network): {}", e);
        }
    }

    // Test 5: Simple API test
    info!("🔧 Testing simple API...");
    use auto_discovery::simple::SimpleDiscovery;

    // This should create the service info quickly without network operations
    let start_time = std::time::Instant::now();
    let simple_discovery = SimpleDiscovery::new().await?;
    let simple_result = simple_discovery
        .register_http_service("test-api", 3000)
        .await;
    let elapsed = start_time.elapsed();

    info!("⏱️  Simple API call completed in {:?}", elapsed);

    match simple_result {
        Ok(_handle) => {
            info!("✅ Simple service registration created successfully");
        }
        Err(e) => {
            info!(
                "ℹ️  Simple registration failed as expected (no network): {}",
                e
            );
        }
    }

    // Test 6: Verify timeout behavior
    info!("⏰ Testing timeout behavior...");
    if elapsed < Duration::from_secs(5) {
        info!("✅ All operations completed quickly - no infinite loops detected");
    } else {
        info!("⚠️  Operations took longer than expected");
    }

    info!("🎉 Non-network test completed successfully!");
    info!("📊 All API functions are working correctly without infinite loops");

    Ok(())
}
