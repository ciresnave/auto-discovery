//! # Auto Discovery
//! 
//! A production-ready service discovery library for Rust applications.
//! 
//! This crate provides a robust, secure, and extensible service discovery solution
//! supporting multiple protocols (mDNS, DNS-SD, UPnP) with production-grade features
//! including caching, health monitoring, metrics, and security.
//! 
//! ## Features
//! 
//! - Multiple protocol support (mDNS, DNS-SD, UPnP)
//! - Async-first design using Tokio
//! - Production safety features (caching, rate limiting, health checks)
//! - Comprehensive security (TSIG, TLS, certificate pinning)
//! - Prometheus metrics integration
//! - Cross-platform support (Windows, Linux, macOS)
//! 
//! ## Quick Start
//! 
//! ```rust
//! use auto_discovery::{
//!     config::DiscoveryConfig,
//!     discovery::ServiceDiscovery,
//!     types::{ProtocolType, ServiceType},
//! };
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create configuration
//!     let config = DiscoveryConfig::default();
//!     
//!     // Initialize service discovery
//!     let discovery = ServiceDiscovery::new(config)?;
//!     
//!     // Discover HTTP services
//!     let services = discovery
//!         .discover_services(vec![
//!             ServiceType::new("_http._tcp")
//!         ])
//!         .await?;
//!     
//!     println!("Found {} services", services.len());
//!     Ok(())
//! }
//! ```
//! - Protocol manager with selective protocol enabling
//! - Cross-platform implementation (Windows, Linux, macOS)
//! - Asynchronous API with tokio support
//! - Builder patterns for type-safe configuration
//! - Secure service verification with cryptographic signatures
//! - Protocol-agnostic service interface
//!
//! ## Advanced Usage
//!
//! ```rust
//! use auto_discovery::{
//!     config::DiscoveryConfig,
//!     service::ServiceInfo,
//!     types::ServiceType,
//! };
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Configure discovery with builder pattern
//!     let config = DiscoveryConfig::builder()
//!         .discovery_interval(Duration::from_secs(5))
//!         .verify_services(true)
//!         .build();
//!     
//!     // Initialize service discovery
//!     let mut discovery = ServiceDiscovery::new(config)?;
//!     
//!     // Register our service
//!     let service = ServiceInfo::new(
//!         "My Service Instance", 
//!         "_myservice._tcp", 
//!         8080,
//!         Some(vec![("version", "1.0"), ("feature", "basic")])
//!     );
//!     discovery.register_service(service).await?;
//!     
//!     // Discover services
//!     let service_types = vec![ServiceType::new("_myservice._tcp")];
//!     let services = discovery
//!         .discover_services(service_types)
//!         .await?;
//!     
//!     for service in services {
//!         println!("Found service: {} at {}:{}", 
//!             service.name,
//!             service.address,
//!             service.port);
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Protocol Manager
//!
//! The library uses a protocol manager to handle multiple discovery protocols:
//!
//! ```rust
//! use auto_discovery::protocols::ProtocolManagerBuilder;
//!
//! let manager = ProtocolManagerBuilder::new(config)
//!     .with_mdns(true)    // Enable mDNS
//!     .with_upnp(true)    // Enable UPnP
//!     .with_dns_sd(false) // Disable DNS-SD
//!     .build()
//!     .await?;
//! ```
//!
//! ## Error Handling
//!
//! The library provides detailed error context for each protocol:
//!
//! ```rust
//! use auto_discovery::error::DiscoveryError;
//!
//! // Protocol-specific errors include the protocol name
//! let err = DiscoveryError::protocol("mdns", "Service registration failed");
//! ```
//!
//! ## Security Features
//!
//! ### TSIG Authentication
//!
//! The library provides comprehensive TSIG (Transaction SIGnature) support for secure DNS updates:
//!
//! ```rust
//! use auto_discovery::{
//!     config::DiscoveryConfig,
//!     security::tsig::{TsigKey, TsigAlgorithm, TsigKeyManager},
//!     discovery::ServiceDiscovery,
//! };
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create TSIG key with expiry
//!     let key = TsigKey::new(
//!         "update.example.com.",
//!         TsigAlgorithm::HmacSha256,
//!         b"secret-key-data",
//!         Some(Duration::from_secs(3600)), // 1 hour validity
//!     ).unwrap();
//!
//!     // Create key manager with rotation
//!     let key_manager = TsigKeyManager::new(Duration::from_secs(300)); // 5 minute rotation check
//!     key_manager.add_key(key).await;
//!
//!     // Start key rotation
//!     key_manager.clone().start_key_rotation().await;
//!
//!     // Create discovery instance with TSIG support
//!     let config = DiscoveryConfig::builder()
//!         .enable_tsig(true)
//!         .tsig_key_manager(key_manager)
//!         .build();
//!
//!     let discovery = ServiceDiscovery::new(config).await.unwrap();
//! }
//! ```
//!
//! Features:
//! - Multiple HMAC algorithms (SHA1, SHA256, SHA384, SHA512)
//! - Automatic key rotation
//! - Key expiry management
//! - Metrics collection
//! - Prometheus integration
//!
//! ### Metrics
//!
//! TSIG-related metrics available:
//! - `autodiscovery_tsig_keys_total`: Number of active TSIG keys
//! - `autodiscovery_tsig_keys_expired_total`: Number of expired keys removed
//! - `autodiscovery_tsig_sign_duration_seconds`: Time taken to sign messages
//! - `autodiscovery_tsig_verify_duration_seconds`: Time taken to verify messages
//! - `autodiscovery_tsig_sign_errors_total`: Number of signing failures
//! - `autodiscovery_tsig_verify_errors_total`: Number of verification failures
//!
//! See the `examples/` directory for more complete examples.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod config;
pub mod discovery;
pub mod error;
pub mod protocols;
pub mod registry;  // Service registry for managing discovered and registered services
pub mod service;
pub mod simple;  // Simple API for common use cases
pub mod types;
pub mod utils;
pub mod security;

// Re-export main types for convenience
pub use config::DiscoveryConfig;
pub use discovery::ServiceDiscovery;
pub use error::{DiscoveryError, Result};
pub use service::{ServiceInfo, ServiceEvent};
pub use types::{ServiceType, ProtocolType};
