# Auto Discovery

A production-grade network and system service discovery library for Rust applications that provides an extensible, async-first solution for automatically detecting, connecting to, and coordinating with other services in a network environment.

[![Crates.io](https://img.shields.io/crates/v/auto-discovery.svg)](https://crates.io/crates/auto-discovery)
[![Documentation](https://docs.rs/auto-discovery/badge.svg)](https://docs.rs/auto-discovery)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/your-org/auto-discovery)

## Features

- 🔍 **Zero-configuration network service discovery**
- 🌐 **Production-grade protocol implementations** (mDNS, DNS-SD, UPnP/SSDP)
- 🛡️ **Comprehensive safety features**
  - Rate limiting
  - Automatic retries
  - Circuit breakers
  - Timeouts
  - Health monitoring
- ⚖️ **Smart load balancing**
  - Multiple strategies (Round Robin, Least Loaded, Random)
  - Response time monitoring
  - Automatic failover
- 📊 **Metrics and monitoring**
  - Prometheus integration
  - Health status tracking
  - Performance metrics
  - Request tracing
- 🔌 **Protocol manager with selective protocol enabling**
- 💻 **Cross-platform implementation** (Windows, Linux, macOS)
- ⚡ **Asynchronous API** with tokio support
- 🔒 **Secure service verification** with cryptographic signatures

## Production Safety Features

### Rate Limiting and Retry

```rust
use auto_discovery::safety::{SafetyConfig, SafetyManager};

// Configure safety features
let safety_config = SafetyConfig {
    rate_limit_per_second: 10,
    retry_max_attempts: 3,
    retry_initial_interval: Duration::from_millis(100),
    health_check_interval: Duration::from_secs(1),
    operation_timeout: Duration::from_secs(5),
};

let safety_manager = SafetyManager::new(safety_config);

// Use with automatic retry
let result = safety_manager.with_retry(|| Box::pin(async {
    manager.register_service(service.clone()).await
})).await?;

// Rate limit check
safety_manager.check_rate_limit(&service_type).await?;
```

### Load Balancing

```rust
use auto_discovery::safety::{LoadBalancer, LoadBalancerConfig, LoadBalancingStrategy};

// Configure load balancer
let lb_config = LoadBalancerConfig {
    strategy: LoadBalancingStrategy::LeastLoaded,
    decay_time: Duration::from_secs(10),
    rtt_threshold: Duration::from_millis(100),
};

let balancer = LoadBalancer::new(lb_config);

// Add services to load balancer
balancer.update_service(service.clone(), 0.0).await?;

// Select optimal service
let selected = balancer.select_service()
    .expect("Should have services available");

// Record metrics
balancer.record_request(
    &selected.id(),
    Duration::from_millis(50),
    true
);
```

### Health Monitoring

```rust
use auto_discovery::safety::{HealthMonitor, ServiceStatus};

let health_monitor = HealthMonitor::new();

// Update service health
health_monitor.update_service(&service, is_healthy);

// Check service status
if let Some(status) = health_monitor.get_service_status(&service.id()) {
    match status {
        ServiceStatus::Healthy => println!("Service is healthy"),
        ServiceStatus::Degraded => println!("Service is degraded"),
        ServiceStatus::Unhealthy => println!("Service is unhealthy"),
    }
}

// Clean up stale entries
health_monitor.cleanup_stale(Duration::from_secs(300));
```

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
auto-discovery = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## Basic Usage

```rust
use auto_discovery::{
    config::DiscoveryConfig,
    protocols::ProtocolManagerBuilder,
    service::ServiceInfo,
    types::ServiceType,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure discovery with builder pattern
    let config = DiscoveryConfig::builder()
        .discovery_interval(Duration::from_secs(5))
        .verify_services(true)
        .build();
    
    // Create protocol manager with selected protocols
    let mut manager = ProtocolManagerBuilder::new(config)
        .with_mdns(true)
        .with_upnp(true)
        .build()
        .await?;
    
    // Register our service
    let service = ServiceInfo::new(
        "my_service",
        "_http._tcp",
        8080,
        Some(vec![("version", "1.0")]),
    );
    manager.register_service(service).await?;
    
    // Discover services
    let service_types = vec![ServiceType::new("_http._tcp")?];
    let services = manager
        .discover_services(service_types, Duration::from_secs(5))
        .await?;
    
    // Process discovered services
    for service in services {
        println!("Found service: {} at {}:{}",
            service.name(),
            service.address(),
            service.port());
    }
    
    Ok(())
}
```

## Protocol Management

The library uses a protocol manager to handle multiple discovery protocols:

```rust
use auto_discovery::protocols::ProtocolManagerBuilder;

// Create a manager with selected protocols
let manager = ProtocolManagerBuilder::new(config)
    .with_mdns(true)    // Enable mDNS
    .with_upnp(true)    // Enable UPnP
    .with_dns_sd(false) // Disable DNS-SD
    .build()
    .await?;

// Discover services across all enabled protocols
let services = manager
    .discover_services(service_types, timeout)
    .await?;
```

## Configuration

Use the builder pattern for type-safe configuration:

```rust
use auto_discovery::config::DiscoveryConfig;

let config = DiscoveryConfig::builder()
    .discovery_interval(Duration::from_secs(30))
    .verify_services(true)
    .network_interface(NetworkInterface::new("eth0"))
    .build();
```

## Examples

The crate includes several examples demonstrating different features:

- `builder_pattern.rs`: Using the builder pattern for configuration
- `cross_protocol.rs`: Working with multiple protocols
- `basic_usage.rs`: Simple service discovery
- `security_verification.rs`: Service verification features

Run an example with:

```bash
cargo run --example builder_pattern
```

## Protocol Support

The library currently supports the following protocols:

- **mDNS**: Multicast DNS for local network service discovery
  - Service registration and browsing
  - TXT record attributes
  - Service type filtering

- **UPnP**: Universal Plug and Play for device discovery
  - SSDP discovery
  - Device description parsing
  - Service verification

- **DNS-SD**: DNS Service Discovery (in development)
  - Service registration
  - Wide-area discovery
  - TXT record support

## Testing

Run the test suite:

```bash
cargo test
```

This includes:

- Unit tests for individual components
- Integration tests for cross-protocol functionality
- Mock implementations for testing without network access

## Advanced Features

### Service Verification

```rust
// Enable service verification in config
let config = DiscoveryConfig::builder()
    .verify_services(true)
    .build();

// Verify a specific service
let verified = manager.verify_service(&service).await?;
```

### Cross-Protocol Discovery

```rust
// Discover services across multiple protocols
let service_types = vec![
    ServiceType::new("_http._tcp")?,           // mDNS
    ServiceType::new("urn:my-service:1")?,     // UPnP
];

let services = manager
    .discover_services(service_types, timeout)
    .await?;
```

## Documentation

For detailed documentation and API reference, visit [docs.rs/auto-discovery](https://docs.rs/auto-discovery).

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate and follow the existing code style.

## License

Licensed under either of:

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))

at your option.

## Acknowledgments

- Inspired by the service discovery needs of the COAD project
- Built with the excellent Rust ecosystem libraries
- Thanks to all contributors and the Rust community

---

For more detailed documentation, visit [docs.rs/auto-discovery](https://docs.rs/auto-discovery).
