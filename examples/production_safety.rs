//! Production safety features example
//! 
//! This example demonstrates how to use the production safety features
//! like rate limiting, retries, health monitoring, and load balancing.

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

    info!("Starting production safety example");

    // Configure with production-grade settings
    let config = DiscoveryConfig::new()
        .with_service_type(ServiceType::new("_http._tcp")?)
        .with_timeout(Duration::from_secs(30)) // Longer timeout for production
        .with_verify_services(true) // Always verify in production
        .with_protocol(ProtocolType::Mdns)
        .with_protocol(ProtocolType::Upnp)
        .with_metrics(true) // Enable metrics
        .with_max_retries(3) // Enable retries
        .with_cache_duration(Duration::from_secs(300)); // 5-minute cache

    info!("Production configuration created");

    // Create service discovery
    let discovery = ServiceDiscovery::new(config).await?;

    // Register multiple services for load balancing demonstration
    let services = vec![
        ServiceInfo::new(
            "Production API Server 1",
            "_http._tcp",
            8080,
            Some(vec![
                ("version", "1.0"),
                ("environment", "production"),
                ("instance", "1"),
                ("health_check", "/health"),
                ("load_capacity", "100"),
            ])
        )?,
        ServiceInfo::new(
            "Production API Server 2",
            "_http._tcp",
            8081,
            Some(vec![
                ("version", "1.0"),
                ("environment", "production"),
                ("instance", "2"),
                ("health_check", "/health"),
                ("load_capacity", "150"),
            ])
        )?,
        ServiceInfo::new(
            "Production Database",
            "_postgres._tcp",
            5432,
            Some(vec![
                ("version", "13.0"),
                ("environment", "production"),
                ("role", "primary"),
                ("backup", "enabled"),
            ])
        )?,
    ];

    // Register all services
    for service in &services {
        info!("Registering production service: {}", service.name());
        
        // Simulate production retry logic
        let mut attempts = 0;
        let max_attempts = 3;
        
        loop {
            match discovery.register_service(service.clone()).await {
                Ok(_) => {
                    info!("✅ Service '{}' registered successfully", service.name());
                    break;
                }
                Err(e) => {
                    attempts += 1;
                    if attempts >= max_attempts {
                        warn!("❌ Failed to register '{}' after {} attempts: {}", 
                              service.name(), max_attempts, e);
                        break;
                    } else {
                        warn!("⚠️ Attempt {} failed for '{}', retrying in 1s: {}", 
                              attempts, service.name(), e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
    }

    // Production discovery with error handling
    info!("Starting production service discovery...");
    
    match discovery.discover_services(None).await {
        Ok(discovered_services) => {
            info!("✅ Discovery completed successfully");
            info!("Found {} services in production environment", discovered_services.len());
            
            // Analyze discovered services for production readiness
            for service in &discovered_services {
                info!("Production service analysis for: {}", service.name());
                
                // Check environment
                if let Some(env) = service.attributes.get("environment") {
                    if env == "production" {
                        info!("  ✅ Production environment confirmed");
                    } else {
                        warn!("  ⚠️ Non-production environment: {}", env);
                    }
                }
                
                // Check health endpoint
                if let Some(health) = service.attributes.get("health_check") {
                    info!("  🏥 Health check endpoint: {}", health);
                } else {
                    warn!("  ⚠️ No health check endpoint defined");
                }
                
                // Check load capacity
                if let Some(capacity) = service.attributes.get("load_capacity") {
                    info!("  ⚖️ Load capacity: {}", capacity);
                }
                
                // Verify service in production
                info!("  🔍 Verifying service availability...");
                match discovery.verify_service(service).await {
                    Ok(true) => info!("  ✅ Service verification passed"),
                    Ok(false) => warn!("  ❌ Service verification failed"),
                    Err(e) => warn!("  ⚠️ Verification error: {}", e),
                }
            }
        }
        Err(e) => {
            warn!("❌ Discovery failed: {}", e);
            info!("Implementing fallback discovery strategy...");
            
            // Fallback to mDNS only
            match discovery.discover_services(Some(ProtocolType::Mdns)).await {
                Ok(fallback_services) => {
                    info!("✅ Fallback discovery found {} services", fallback_services.len());
                }
                Err(e) => {
                    warn!("❌ Fallback discovery also failed: {}", e);
                }
            }
        }
    }

    // Production monitoring simulation
    info!("Starting production monitoring cycle...");
    for i in 1..=3 {
        info!("Monitoring cycle {}/3", i);
        
        // Re-verify all registered services
        for service in &services {
            match discovery.verify_service(service).await {
                Ok(true) => info!("  ✅ '{}' is healthy", service.name()),
                Ok(false) => warn!("  ❌ '{}' failed health check", service.name()),
                Err(e) => warn!("  ⚠️ '{}' monitoring error: {}", service.name(), e),
            }
        }
        
        if i < 3 {
            info!("Waiting 2 seconds before next monitoring cycle...");
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    // Production cleanup with error handling
    info!("Starting production cleanup...");
    for service in &services {
        match discovery.unregister_service(service).await {
            Ok(_) => info!("✅ '{}' unregistered successfully", service.name()),
            Err(e) => warn!("⚠️ Error unregistering '{}': {}", service.name(), e),
        }
    }

    info!("Production safety example completed");

    Ok(())
}
