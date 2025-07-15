// Example: Using both simple and advanced APIs
//
// This example demonstrates the flexibility of the AutoDiscovery crate:
// 1. Simple one-liner API for common use cases
// 2. Advanced filtering API for complex scenarios

use auto_discovery::{
    simple::{discover_http_services, register_http_service},
    ServiceDiscovery, DiscoveryConfig,
    types::{ProtocolType, ServiceType}
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("🔍 AutoDiscovery Crate - API Demonstration");
    
    // ==============================================
    // PART 1: Simple API - For Beginners
    // ==============================================
    println!("\n🚀 SIMPLE API USAGE:");
    
    // One-liner service registration
    println!("📝 Registering HTTP service with simple API...");
    register_http_service("demo-api", 8080).await?;
    
    // Wait a moment for registration
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // One-liner service discovery  
    println!("🔍 Discovering HTTP services with simple API...");
    let simple_services = discover_http_services().await?;
    println!("✅ Found {} HTTP services via simple API", simple_services.len());
    
    // ==============================================
    // PART 2: Advanced API - For Complex Use Cases
    // ==============================================
    println!("\n🎯 ADVANCED API USAGE:");
    
    // Advanced configuration
    let config = DiscoveryConfig::new()
        .with_protocol(ProtocolType::Mdns)
        .with_protocol(ProtocolType::Upnp)
        .with_timeout(Duration::from_secs(3))
        .with_service_type(ServiceType::new("_http._tcp.local.")?);
    
    let discovery = ServiceDiscovery::new(config).await?;
    
    // Filtered discovery - only mDNS HTTP services
    println!("🔍 Discovering only mDNS HTTP services...");
    let mdns_services = discovery.discover_services_filtered(
        Some(vec![ServiceType::new("_http._tcp.local.")?]),
        Some(ProtocolType::Mdns)
    ).await?;
    println!("✅ Found {} HTTP services via mDNS", mdns_services.len());
    
    // Filtered discovery - all service types via UPnP
    println!("🔍 Discovering all services via UPnP only...");
    let upnp_services = discovery.discover_services_filtered(
        None, // All configured service types
        Some(ProtocolType::Upnp)
    ).await?;
    println!("✅ Found {} services via UPnP", upnp_services.len());
    
    // All services, all protocols
    println!("🔍 Discovering all services via all protocols...");
    let all_services = discovery.discover_services_filtered(None, None).await?;
    println!("✅ Found {} total services", all_services.len());
    
    // ==============================================
    // PART 3: Service Details
    // ==============================================
    println!("\n📊 SERVICE DETAILS:");
    
    for service in &all_services {
        println!("  🔹 {} [{}:{}] via {:?}", 
            service.name(), 
            service.address(), 
            service.port(),
            service.protocol_type()
        );
        
        // Show attributes if any
        if !service.attributes.is_empty() {
            for (key, value) in &service.attributes {
                println!("     • {}: {}", key, value);
            }
        }
    }
    
    println!("\n✨ API demonstration complete!");
    println!("\n💡 Key Takeaways:");
    println!("   • Simple API: Perfect for quick HTTP service tasks");
    println!("   • Advanced API: Full control over protocols and service types");  
    println!("   • Filtered discovery: Granular control for complex scenarios");
    println!("   • Both APIs work together seamlessly");
    
    Ok(())
}