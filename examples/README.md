# Auto Discovery Examples

This directory contains various examples demonstrating the auto-discovery library functionality.

## Examples Overview

### 🚀 [`basic_usage.rs`](basic_usage.rs)
**Comprehensive introduction to the library**
- Service registration (HTTP and SSH services)
- Multi-protocol discovery (mDNS and UPnP)
- Protocol filtering
- Proper error handling and cleanup
- **Best for**: New users wanting to understand all core features

### 🔍 [`simple_discovery.rs`](simple_discovery.rs)
**Minimal, beginner-friendly example**
- Basic service registration
- Simple service discovery
- Clean resource management
- **Best for**: Quick start and learning the basics

### 🏗️ [`builder_pattern.rs`](builder_pattern.rs)
**Advanced configuration using builder patterns**
- Complex service configuration
- Custom attributes and metadata
- Builder pattern demonstration
- **Best for**: Understanding advanced configuration options

### 🌐 [`cross_protocol.rs`](cross_protocol.rs)
**Multi-protocol service discovery**
- Simultaneous use of multiple discovery protocols
- Protocol-specific service registration
- Cross-protocol compatibility
- **Best for**: Enterprise scenarios with mixed environments

### 🔒 [`security_verification.rs`](security_verification.rs)
**Security features and service verification**
- Service authentication
- TSIG-based verification
- Security policy enforcement
- **Best for**: Security-conscious deployments

### 🏭 [`production_safety.rs`](production_safety.rs)
**Production-ready implementation**
- Comprehensive error handling
- Health monitoring
- Resource management
- Graceful degradation
- **Best for**: Production deployments

### 🔐 [`multi_protocol_security.rs`](multi_protocol_security.rs)
**Advanced security across multiple protocols**
- Multi-protocol security policies
- Advanced authentication
- Security auditing
- **Best for**: High-security environments

### 🌍 [`dns_sd_example.rs`](dns_sd_example.rs)
**DNS Service Discovery specific example**
- DNS-SD protocol usage
- Domain-specific service registration
- DNS-SD specific features
- **Best for**: DNS-SD specific deployments

## Running Examples

To run any example:

```bash
# Run a specific example
cargo run --example basic_usage

# Run with debug output
RUST_LOG=debug cargo run --example simple_discovery

# Build all examples (check for compilation errors)
cargo check --examples
```

## Example Progression

For learning the library, we recommend this progression:

1. **Start here**: `simple_discovery.rs` - Learn the basics
2. **Core features**: `basic_usage.rs` - Understand all main functionality
3. **Advanced config**: `builder_pattern.rs` - Learn configuration patterns
4. **Multi-protocol**: `cross_protocol.rs` - Understand protocol integration
5. **Security**: `security_verification.rs` - Add security features
6. **Production**: `production_safety.rs` - Production deployment patterns

## Common Patterns

### Service Registration
```rust
let service = ServiceInfo::new(
    "My Service",
    "_http._tcp",
    8080,
    Some(vec![("version", "1.0")])
)?;

discovery.register_service(service).await?;
```

### Service Discovery
```rust
// Discover all services
let services = discovery.discover_services(None).await?;

// Filter by protocol
let mdns_services = discovery.discover_services(Some(ProtocolType::Mdns)).await?;
```

### Configuration
```rust
let config = DiscoveryConfig::new()
    .with_service_type(ServiceType::new("_http._tcp")?)
    .with_protocol(ProtocolType::Mdns)
    .with_timeout(Duration::from_secs(5));
```

## Troubleshooting

- **Service not found**: Check network connectivity and firewall settings
- **Registration fails**: Verify service type format and port availability
- **Timeout errors**: Increase timeout in configuration
- **Permission issues**: Run with appropriate network permissions

For more detailed documentation, see the [main project README](../README.md).