//! Graceful shutdown handling for service discovery

use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{Duration, sleep, timeout};
use tracing::{debug, info, warn, error};
use futures::future::join_all;
use crate::{
    error::{Result, DiscoveryError},
    protocols::ProtocolManager,
    service::ServiceInfo,
    safety::SafetyManager,
};

/// Maximum time to wait for graceful shutdown
const MAX_SHUTDOWN_WAIT: Duration = Duration::from_secs(30);
/// Time to wait between shutdown steps
const SHUTDOWN_STEP_DELAY: Duration = Duration::from_millis(100);
/// Maximum number of unregister retries
const MAX_UNREGISTER_RETRIES: u32 = 3;

/// Shutdown stages for ordered cleanup
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShutdownStage {
    /// Stop accepting new service registrations
    StopRegistrations,
    /// Unregister active services
    UnregisterServices,
    /// Stop discovery protocols
    StopProtocols,
    /// Final cleanup
    Cleanup,
}

/// Status of the shutdown process
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShutdownStatus {
    /// Current shutdown stage
    pub stage: String,
    /// Number of services remaining to unregister
    pub remaining_services: usize,
    /// Whether the shutdown completed successfully
    pub success: bool,
}

/// Shutdown manager for graceful service termination
pub struct ShutdownManager {
    /// Protocol manager reference
    protocol_manager: Arc<RwLock<ProtocolManager>>,
    
    /// Safety manager reference
    safety_manager: Arc<SafetyManager>,
    
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
    
    /// Active services
    active_services: Arc<RwLock<Vec<ServiceInfo>>>,
    
    /// Shutdown status
    status: Arc<RwLock<ShutdownStatus>>,
}

impl ShutdownManager {
    /// Create a new shutdown manager
    pub fn new(
        protocol_manager: Arc<RwLock<ProtocolManager>>,
        safety_manager: Arc<SafetyManager>,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        
        Self {
            protocol_manager,
            safety_manager,
            shutdown_tx,
            active_services: Arc::new(RwLock::new(Vec::new())),
            status: Arc::new(RwLock::new(ShutdownStatus {
                stage: "Initialized".to_string(),
                remaining_services: 0,
                success: true,
            })),
        }
    }

    /// Get a shutdown signal receiver
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Add a service to track for shutdown
    pub async fn track_service(&self, service: ServiceInfo) {
        let mut services = self.active_services.write().await;
        services.push(service);
    }

    /// Remove a service from tracking
    pub async fn untrack_service(&self, service: &ServiceInfo) {
        let mut services = self.active_services.write().await;
        services.retain(|s| s.id != service.id);
    }

    /// Initiate graceful shutdown
    pub async fn shutdown(&self) -> Result<()> {
        info!("Initiating graceful shutdown");
        
        // Broadcast shutdown signal
        let _ = self.shutdown_tx.send(());
        
        // Execute shutdown stages
        let stages = [
            (ShutdownStage::StopRegistrations, self.stop_registrations()),
            (ShutdownStage::UnregisterServices, self.unregister_services()),
            (ShutdownStage::StopProtocols, self.stop_protocols()),
            (ShutdownStage::Cleanup, self.cleanup()),
        ];

        for (stage, action) in stages {
            let stage_str = format!("{:?}", stage);
            self.update_status(stage_str.clone(), true).await;
            
            if let Err(e) = timeout(MAX_SHUTDOWN_WAIT, action).await {
                error!("Shutdown stage {} timed out: {}", stage_str, e);
                self.update_status(stage_str, false).await;
                return Err(DiscoveryError::Timeout(format!("Shutdown stage {} timed out", stage_str)));
            }
            
            sleep(SHUTDOWN_STEP_DELAY).await;
        }

        info!("Graceful shutdown completed successfully");
        Ok(())
    }

    /// Get current shutdown status
    pub async fn get_status(&self) -> ShutdownStatus {
        self.status.read().await.clone()
    }

    /// Update shutdown status
    async fn update_status(&self, stage: String, success: bool) {
        let mut status = self.status.write().await;
        status.stage = stage;
        status.success = success;
    }

    /// Stop accepting new service registrations
    async fn stop_registrations(&self) -> Result<()> {
        let pm = self.protocol_manager.read().await;
        pm.stop_registrations().await?;
        debug!("Stopped accepting new service registrations");
        Ok(())
    }

    /// Unregister all active services
    async fn unregister_services(&self) -> Result<()> {
        let mut services = self.active_services.write().await;
        let mut pm = self.protocol_manager.write().await;
        
        let mut remaining = services.len();
        self.update_remaining(remaining).await;
        
        for service in services.drain(..) {
            for attempt in 1..=MAX_UNREGISTER_RETRIES {
                match pm.unregister_service(&service).await {
                    Ok(_) => {
                        remaining -= 1;
                        self.update_remaining(remaining).await;
                        break;
                    }
                    Err(e) if attempt < MAX_UNREGISTER_RETRIES => {
                        warn!("Failed to unregister service {} (attempt {}): {}", service.name, attempt, e);
                        sleep(SHUTDOWN_STEP_DELAY).await;
                    }
                    Err(e) => {
                        error!("Failed to unregister service {} after {} attempts: {}", service.name, MAX_UNREGISTER_RETRIES, e);
                    }
                }
            }
        }
        
        debug!("Unregistered all active services");
        Ok(())
    }

    /// Update remaining services count in status
    async fn update_remaining(&self, remaining: usize) {
        let mut status = self.status.write().await;
        status.remaining_services = remaining;
    }

    /// Stop all discovery protocols
    async fn stop_protocols(&self) -> Result<()> {
        let mut pm = self.protocol_manager.write().await;
        pm.stop_all_protocols().await?;
        debug!("Stopped all discovery protocols");
        Ok(())
    }

    /// Final cleanup
    async fn cleanup(&self) -> Result<()> {
        // Ensure safety systems are properly shut down
        self.safety_manager.shutdown().await?;
        
        // Clear any remaining state
        let mut services = self.active_services.write().await;
        services.clear();
        
        debug!("Cleanup completed");
        Ok(())
    }
}

impl Clone for ShutdownManager {
    fn clone(&self) -> Self {
        Self {
            protocol_manager: self.protocol_manager.clone(),
            safety_manager: self.safety_manager.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
            active_services: self.active_services.clone(),
            status: self.status.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::DiscoveryConfig,
        protocols::ProtocolManagerBuilder,
        safety::SafetyConfig,
    };
    use std::time::Duration;

    #[tokio::test]
    async fn test_shutdown_manager() {
        // Setup test components
        let config = DiscoveryConfig::default();
        let protocol_manager = Arc::new(RwLock::new(
            ProtocolManagerBuilder::new(config).build().await.unwrap()
        ));
        
        let safety_manager = Arc::new(SafetyManager::new(SafetyConfig::default()));
        
        let shutdown_manager = ShutdownManager::new(
            protocol_manager.clone(),
            safety_manager.clone(),
        );

        // Add test services
        let service = ServiceInfo::new(
            "test-service",
            "_test._tcp",
            8080,
            None,
        );
        
        shutdown_manager.track_service(service.clone()).await;

        // Test shutdown
        let result = shutdown_manager.shutdown().await;
        assert!(result.is_ok());

        // Verify services were unregistered
        let services = shutdown_manager.active_services.read().await;
        assert!(services.is_empty());
    }

    #[tokio::test]
    async fn test_shutdown_timeout() {
        // Setup test components with slow unregistration
        let config = DiscoveryConfig {
            service_timeout: Duration::from_secs(40), // Longer than MAX_SHUTDOWN_WAIT
            ..Default::default()
        };
        
        let protocol_manager = Arc::new(RwLock::new(
            ProtocolManagerBuilder::new(config).build().await.unwrap()
        ));
        
        let safety_manager = Arc::new(SafetyManager::new(SafetyConfig::default()));
        
        let shutdown_manager = ShutdownManager::new(
            protocol_manager.clone(),
            safety_manager.clone(),
        );

        // Add test service
        let service = ServiceInfo::new(
            "slow-service",
            "_test._tcp",
            8080,
            None,
        );
        
        shutdown_manager.track_service(service).await;

        // Test shutdown (should complete with timeout)
        let start = std::time::Instant::now();
        let result = shutdown_manager.shutdown().await;
        
        assert!(result.is_ok());
        assert!(start.elapsed() <= MAX_SHUTDOWN_WAIT + Duration::from_secs(1));
    }
}
