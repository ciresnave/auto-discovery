use auto_discovery::{
    config::DiscoveryConfig,
    error::Result,
    protocols::{upnp::SsdpProtocol, DiscoveryProtocol},
    service::ServiceInfo,
    types::{ProtocolType, ServiceType},
};
use std::{net::IpAddr, str::FromStr, time::Duration};
use tokio::time;

#[tokio::test]
async fn test_ssdp_protocol_lifecycle() -> Result<()> {
    let config = DiscoveryConfig::default();
    let ssdp = SsdpProtocol::new(&config).await?;
    
    assert_eq!(ssdp.protocol_type(), ProtocolType::Upnp);
    assert!(ssdp.is_available().await);
    
    Ok(())
}

#[tokio::test]
async fn test_ssdp_service_registration() -> Result<()> {
    let config = DiscoveryConfig::default();
    let ssdp = SsdpProtocol::new(&config).await?;
    
    let service = ServiceInfo::new(
        "test-ssdp-service",
        "urn:test-service-type",
        8080,
        Some(vec![("version", "1.0")])
    )?
    .with_address(IpAddr::from_str("127.0.0.1").map_err(|e| auto_discovery::error::DiscoveryError::network(e.to_string()))?)
    .with_protocol_type(ProtocolType::Upnp);
    
    // Register service
    ssdp.register_service(service.clone()).await?;
    
    // Allow time for registration
    time::sleep(Duration::from_millis(100)).await;
    
    // Verify service is discoverable
    let discovered = ssdp.discover_services(
        vec![ServiceType::new("urn:test-service-type")?],
        Some(Duration::from_secs(1))
    ).await?;
    
    assert!(!discovered.is_empty());
    
    // Cleanup
    ssdp.unregister_service(&service).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_ssdp_service_verification() -> Result<()> {
    let config = DiscoveryConfig::default();
    let ssdp = SsdpProtocol::new(&config).await?;
    
    let service = ServiceInfo::new(
        "test-verify-service",
        "urn:test-service-type",
        8081,
        Some(vec![("version", "1.0")])
    )?
    .with_address(IpAddr::from_str("127.0.0.1").map_err(|e| auto_discovery::error::DiscoveryError::network(e.to_string()))?)
    .with_protocol_type(ProtocolType::Upnp);
    
    ssdp.register_service(service.clone()).await?;
    
    // Verify service is alive
    assert!(ssdp.verify_service(&service).await?);
    
    // Cleanup
    ssdp.unregister_service(&service).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_ssdp_timeout_handling() -> Result<()> {
    let config = DiscoveryConfig::default();
    let ssdp = SsdpProtocol::new(&config).await?;
    
    // Use very short timeout for non-existent service
    let services = ssdp.discover_services(
        vec![ServiceType::new("urn:nonexistent-service")?],
        Some(Duration::from_millis(100))
    ).await?;
    
    assert!(services.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_ssdp_multiple_services() -> Result<()> {
    let config = DiscoveryConfig::default();
    let ssdp = SsdpProtocol::new(&config).await?;
    
    let mut services = Vec::new();
    for i in 1..=3 {
        let service = ServiceInfo::new(
            &format!("test-service-{}", i),
            "urn:test-service-type",
            8080 + i as u16,
            Some(vec![("instance", &i.to_string())])
        )?
        .with_address(IpAddr::from_str("127.0.0.1").map_err(|e| auto_discovery::error::DiscoveryError::network(e.to_string()))?)
        .with_protocol_type(ProtocolType::Upnp);
        
        services.push(service);
    }
    
    // Register all services
    for service in &services {
        ssdp.register_service(service.clone()).await?;
    }
    
    // Allow time for registration
    time::sleep(Duration::from_millis(100)).await;
    
    // Discover services
    let discovered = ssdp.discover_services(
        vec![ServiceType::new("urn:test-service-type")?],
        Some(Duration::from_secs(1))
    ).await?;
    
    assert_eq!(discovered.len(), services.len());
    
    // Cleanup
    for service in &services {
        ssdp.unregister_service(service).await?;
    }
    
    Ok(())
}

#[tokio::test]
async fn test_ssdp_rate_limiting() -> Result<()> {
    let config = DiscoveryConfig::default();
    let ssdp = SsdpProtocol::new(&config).await?;
    
    let service = ServiceInfo::new(
        "rate-limit-test",
        "urn:test-service-type",
        8082,
        None
    )?
    .with_address(IpAddr::from_str("127.0.0.1").map_err(|e| auto_discovery::error::DiscoveryError::network(e.to_string()))?)
    .with_protocol_type(ProtocolType::Upnp);
    
    // Try to register the same service multiple times quickly
    for _ in 0..5 {
        let _ = ssdp.register_service(service.clone()).await;
    }
    
    // Should still work
    assert!(ssdp.verify_service(&service).await?);
    
    // Cleanup
    ssdp.unregister_service(&service).await?;
    
    Ok(())
}

#[tokio::test]
async fn test_ssdp_invalid_service() -> Result<()> {
    let config = DiscoveryConfig::default();
    let ssdp = SsdpProtocol::new(&config).await?;
    
    // Try to create an invalid service (empty service type should cause an error)
    let invalid_service_result = ServiceInfo::new(
        "invalid-service",
        "",  // Empty service type should cause validation error
        8083,
        None
    );
    
    assert!(invalid_service_result.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_ssdp_concurrent_operations() -> Result<()> {
    let config = DiscoveryConfig::default();
    let ssdp = std::sync::Arc::new(SsdpProtocol::new(&config).await?);
    
    let mut handles = Vec::new();
    
    // Spawn multiple concurrent registration tasks
    for i in 0..3 {
        let ssdp_clone = ssdp.clone();
        let handle = tokio::spawn(async move {
            let service = ServiceInfo::new(
                &format!("concurrent-service-{}", i),
                "urn:test-service-type",
                8084 + i as u16,
                None
            ).unwrap()
            .with_address(IpAddr::from_str("127.0.0.1").unwrap())
            .with_protocol_type(ProtocolType::Upnp);
            
            ssdp_clone.register_service(service).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await;
        // Each task should complete successfully
        assert!(result.is_ok());
    }
    
    Ok(())
}
