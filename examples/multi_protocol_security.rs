//! Multi-protocol security example
//! 
//! This example demonstrates security features across multiple protocols
//! and shows how to handle security in a multi-protocol environment.

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

    info!("Starting multi-protocol security example");

    // Configure with security-focused settings
    let config = DiscoveryConfig::new()
        .with_service_type(ServiceType::new("_http._tcp")?)
        .with_service_type(ServiceType::new("_https._tcp")?)
        .with_timeout(Duration::from_secs(15))
        .with_verify_services(true) // Critical for security
        .with_protocol(ProtocolType::Mdns)
        .with_protocol(ProtocolType::Upnp)
        .with_protocol(ProtocolType::DnsSd)
        .with_cross_protocol(true)
        .with_metrics(true);

    info!("Security-focused configuration created");

    // Create service discovery
    let discovery = ServiceDiscovery::new(config).await?;

    // Register services with different security levels
    let secure_services = vec![
        ServiceInfo::new(
            "High Security API",
            "_https._tcp",
            8443,
            Some(vec![
                ("security_level", "high"),
                ("tls_version", "1.3"),
                ("auth_method", "oauth2"),
                ("encryption", "aes256"),
                ("cert_validation", "strict"),
            ])
        )?,
        ServiceInfo::new(
            "Medium Security Service",
            "_https._tcp",
            8444,
            Some(vec![
                ("security_level", "medium"),
                ("tls_version", "1.2"),
                ("auth_method", "basic"),
                ("encryption", "aes128"),
            ])
        )?,
        ServiceInfo::new(
            "Legacy HTTP Service",
            "_http._tcp",
            8080,
            Some(vec![
                ("security_level", "low"),
                ("auth_method", "none"),
                ("legacy", "true"),
                ("deprecation_notice", "migrate_to_https"),
            ])
        )?,
    ];

    // Register services across protocols
    for service in &secure_services {
        info!("Registering secure service: {}", service.name());
        discovery.register_service(service.clone()).await?;
    }

    // Discover services via each protocol and analyze security
    let protocols = vec![ProtocolType::Mdns, ProtocolType::Upnp, ProtocolType::DnsSd];
    
    for protocol in &protocols {
        info!("Discovering services via {:?}...", protocol);
        
        match discovery.discover_services(Some(*protocol)).await {
            Ok(services) => {
                info!("Found {} services via {:?}", services.len(), protocol);
                
                for service in &services {
                    analyze_service_security(service, &discovery).await;
                }
            }
            Err(e) => {
                warn!("Discovery failed for {:?}: {}", protocol, e);
            }
        }
    }

    // Cross-protocol security analysis
    info!("Performing cross-protocol security analysis...");
    
    let all_services = discovery.discover_services(None).await?;
    let mut security_summary = SecuritySummary::new();
    
    for service in &all_services {
        let security_level = service.attributes.get("security_level")
            .map(|s| s.as_str())
            .unwrap_or("unknown");
            
        security_summary.add_service(security_level, service.protocol_type());
        
        // Check for security vulnerabilities
        check_security_vulnerabilities(service);
    }
    
    security_summary.print_report();

    // Security recommendations
    print_security_recommendations(&all_services);

    // Cleanup
    for service in &secure_services {
        discovery.unregister_service(service).await?;
    }

    info!("Multi-protocol security example completed");

    Ok(())
}

async fn analyze_service_security(service: &ServiceInfo, discovery: &ServiceDiscovery) {
    info!("🔍 Security analysis for: {}", service.name());
    
    // Verify service
    match discovery.verify_service(service).await {
        Ok(true) => info!("  ✅ Service verification passed"),
        Ok(false) => warn!("  ❌ Service verification failed"),
        Err(e) => warn!("  ⚠️ Verification error: {}", e),
    }
    
    // Analyze security attributes
    if let Some(security_level) = service.attributes.get("security_level") {
        match security_level.as_str() {
            "high" => info!("  🔒 High security level - RECOMMENDED"),
            "medium" => info!("  🔐 Medium security level - ACCEPTABLE"),
            "low" => warn!("  ⚠️ Low security level - REVIEW REQUIRED"),
            _ => warn!("  ❓ Unknown security level: {}", security_level),
        }
    } else {
        warn!("  ⚠️ No security level specified");
    }
    
    // Check TLS
    if let Some(tls) = service.attributes.get("tls_version") {
        match tls.as_str() {
            "1.3" => info!("  🛡️ TLS 1.3 - EXCELLENT"),
            "1.2" => info!("  🛡️ TLS 1.2 - GOOD"),
            "1.1" | "1.0" => warn!("  ⚠️ TLS {} - DEPRECATED", tls),
            _ => warn!("  ❓ Unknown TLS version: {}", tls),
        }
    }
    
    // Check authentication
    if let Some(auth) = service.attributes.get("auth_method") {
        match auth.as_str() {
            "oauth2" => info!("  🔑 OAuth2 authentication - EXCELLENT"),
            "jwt" => info!("  🔑 JWT authentication - GOOD"),
            "basic" => warn!("  ⚠️ Basic authentication - WEAK"),
            "none" => warn!("  ❌ No authentication - INSECURE"),
            _ => info!("  🔑 Custom authentication: {}", auth),
        }
    }
}

fn check_security_vulnerabilities(service: &ServiceInfo) {
    let mut vulnerabilities = Vec::new();
    
    // Check for common vulnerabilities
    if service.service_type().service_name() == "_http._tcp" {
        vulnerabilities.push("Unencrypted HTTP traffic");
    }
    
    if service.attributes.get("auth_method") == Some(&"none".to_string()) {
        vulnerabilities.push("No authentication required");
    }
    
    if service.attributes.get("legacy") == Some(&"true".to_string()) {
        vulnerabilities.push("Legacy service - may have security issues");
    }
    
    if !vulnerabilities.is_empty() {
        warn!("  🚨 Security vulnerabilities found for '{}':", service.name());
        for vuln in vulnerabilities {
            warn!("    - {}", vuln);
        }
    }
}

struct SecuritySummary {
    high_security: usize,
    medium_security: usize,
    low_security: usize,
    unknown_security: usize,
    protocols: std::collections::HashMap<ProtocolType, usize>,
}

impl SecuritySummary {
    fn new() -> Self {
        Self {
            high_security: 0,
            medium_security: 0,
            low_security: 0,
            unknown_security: 0,
            protocols: std::collections::HashMap::new(),
        }
    }
    
    fn add_service(&mut self, security_level: &str, protocol: ProtocolType) {
        match security_level {
            "high" => self.high_security += 1,
            "medium" => self.medium_security += 1,
            "low" => self.low_security += 1,
            _ => self.unknown_security += 1,
        }
        
        *self.protocols.entry(protocol).or_insert(0) += 1;
    }
    
    fn print_report(&self) {
        info!("📊 Security Summary Report:");
        info!("  High Security Services: {}", self.high_security);
        info!("  Medium Security Services: {}", self.medium_security);
        info!("  Low Security Services: {}", self.low_security);
        info!("  Unknown Security Services: {}", self.unknown_security);
        
        info!("  Services by Protocol:");
        for (protocol, count) in &self.protocols {
            info!("    {:?}: {} services", protocol, count);
        }
    }
}

fn print_security_recommendations(services: &[ServiceInfo]) {
    info!("🔐 Security Recommendations:");
    
    let has_http = services.iter().any(|s| s.service_type().service_name() == "_http._tcp");
    let has_no_auth = services.iter().any(|s| s.attributes.get("auth_method") == Some(&"none".to_string()));
    let has_legacy = services.iter().any(|s| s.attributes.get("legacy") == Some(&"true".to_string()));
    
    if has_http {
        info!("  1. ⚠️ Migrate HTTP services to HTTPS");
        info!("     - Use TLS 1.2 or higher");
        info!("     - Implement proper certificate validation");
    }
    
    if has_no_auth {
        info!("  2. 🔑 Implement authentication for unprotected services");
        info!("     - Use OAuth2, JWT, or similar modern methods");
        info!("     - Avoid basic authentication over unencrypted connections");
    }
    
    if has_legacy {
        info!("  3. 🏗️ Plan migration for legacy services");
        info!("     - Update to modern security standards");
        info!("     - Implement regular security audits");
    }
    
    info!("  4. 🔍 General recommendations:");
    info!("     - Always enable service verification");
    info!("     - Use strong encryption (AES-256)");
    info!("     - Implement certificate pinning where possible");
    info!("     - Monitor services for security changes");
    info!("     - Regular security assessments");
}
