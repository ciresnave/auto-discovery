//! Example demonstrating service security verification features
//! 
//! This example shows how to use the security verification features
//! to ensure discovered services are authentic and trustworthy.

use auto_discovery::{
    config::DiscoveryConfig,
    service::ServiceInfo,
    types::{ServiceType, ProtocolType},
    ServiceDiscovery,
};
use std::time::Duration;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting security verification example");

    // Configure discovery with security verification enabled
    let config = DiscoveryConfig::new()
        .with_timeout(Duration::from_secs(15))
        .with_verify_services(true) // Enable service verification
        .with_service_type(ServiceType::new("_http._tcp")?)
        .with_service_type(ServiceType::new("_https._tcp")?)
        .with_protocol(ProtocolType::Mdns)
        .with_protocol(ProtocolType::Upnp)
        .with_metrics(true); // Enable metrics for monitoring

    info!("Configuration created with security verification enabled");

    // Create service discovery instance
    let discovery = ServiceDiscovery::new(config).await?;

    info!("Service discovery created with security features");

    // Register a secure service
    let secure_service = ServiceInfo::new(
        "Secure Web API",
        "_https._tcp",
        8443,
        Some(vec![
            ("version", "1.0"),
            ("security", "enabled"),
            ("tls", "1.3"),
            ("auth", "required"),
        ])
    )?;

    info!("Registering secure service: {}", secure_service.name());
    discovery.register_service(secure_service.clone()).await?;
    info!("Secure service registered successfully");

    // Register a regular service for comparison
    let regular_service = ServiceInfo::new(
        "Regular Web Server",
        "_http._tcp",
        8080,
        Some(vec![
            ("version", "1.0"),
            ("security", "basic"),
        ])
    )?;

    info!("Registering regular service: {}", regular_service.name());
    discovery.register_service(regular_service.clone()).await?;
    info!("Regular service registered successfully");

    // Discover services
    info!("Discovering services with verification...");
    let services = discovery.discover_services(None).await?;

    info!("Found {} services, verifying each one...", services.len());

    // Verify each discovered service
    for service in &services {
        info!("Verifying service: {}", service.name());
        
        match discovery.verify_service(service).await {
            Ok(is_verified) => {
                if is_verified {
                    info!("✅ Service '{}' is verified and trustworthy", service.name());
                    
                    // Check for security attributes
                    if let Some(security) = service.attributes.get("security") {
                        match security.as_str() {
                            "enabled" => info!("  🔒 High security level detected"),
                            "basic" => info!("  🔐 Basic security level detected"),
                            _ => info!("  ⚠️  Unknown security level: {}", security),
                        }
                    }

                    // Check for TLS support
                    if let Some(tls) = service.attributes.get("tls") {
                        info!("  🛡️  TLS version: {}", tls);
                    }

                    // Check for authentication requirements
                    if let Some(auth) = service.attributes.get("auth") {
                        if auth == "required" {
                            info!("  🔑 Authentication required");
                        }
                    }
                } else {
                    warn!("❌ Service '{}' failed verification", service.name());
                    warn!("  This service may be unreachable or untrustworthy");
                }
            }
            Err(e) => {
                warn!("⚠️  Error verifying service '{}': {}", service.name(), e);
            }
        }
    }

    // Demonstrate re-verification after some time
    info!("Waiting 2 seconds before re-verification...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    info!("Re-verifying services to check for changes...");
    for service in &services {
        match discovery.verify_service(service).await {
            Ok(is_verified) => {
                if is_verified {
                    info!("✅ Service '{}' re-verification successful", service.name());
                } else {
                    warn!("❌ Service '{}' failed re-verification", service.name());
                }
            }
            Err(e) => {
                warn!("⚠️  Error during re-verification of '{}': {}", service.name(), e);
            }
        }
    }

    // Security recommendations
    info!("Security recommendations:");
    info!("  1. Always enable service verification in production");
    info!("  2. Prefer services with TLS support");
    info!("  3. Check service attributes for security indicators");
    info!("  4. Regularly re-verify services in long-running applications");
    info!("  5. Monitor verification failures for potential security issues");

    // Cleanup
    discovery.unregister_service(&secure_service).await?;
    discovery.unregister_service(&regular_service).await?;
    info!("Services unregistered");

    Ok(())
}
