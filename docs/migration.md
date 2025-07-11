# Migration Guide

This document helps you migrate between versions of the auto-discovery crate.

## Migrating to 1.0.0

### Breaking Changes

1. Configuration Builder Pattern
   - Old configuration struct replaced with builder pattern
   - Improved type safety and validation

Before:

```rust
let config = DiscoveryConfig {
    cache_size: 1000,
    ttl: Duration::from_secs(300),
    ..Default::default()
};
```

After:

```rust
let config = DiscoveryConfig::builder()
    .cache_config(
        CacheConfig::builder()
            .max_entries(1000)
            .ttl(Duration::from_secs(300))
            .build()
    )
    .build();
```

1. Protocol-Specific Configuration
   - Separate configuration types for each protocol
   - Better compile-time validation

Before:

```rust
let config = DiscoveryConfig {
    mdns_enabled: true,
    mdns_interface: Some("eth0".to_string()),
    ..Default::default()
};
```

After:

```rust
let mdns_config = MdnsConfig::builder()
    .interface("eth0")
    .build();

let config = DiscoveryConfig::builder()
    .mdns_config(mdns_config)
    .build();
```

1. Error Types
   - More specific error types
   - Better error context and chain

Before:

```rust
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Network error: {0}")]
    Network(String),
}
```

After:

```rust
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("Protocol error: {source}")]
    Protocol {
        #[from]
        source: ProtocolError,
        context: String,
    },
    #[error("Network error: {source}")]
    Network {
        #[from]
        source: std::io::Error,
        context: String,
    },
}
```

### New Features

1. Security Enhancements
   - TSIG authentication for DNS updates
   - Certificate pinning
   - TLS/DTLS support

```rust
let dns_config = DnsConfig::builder()
    .tsig_key(TsigKeyConfig::new(
        "update-key",
        "HMAC-SHA256",
        "base64-encoded-key"
    ))
    .build();
```

2. Metrics and Monitoring
   - Prometheus integration
   - OpenTelemetry support
   - Health check endpoints

```rust
let metrics_config = MetricsConfig::builder()
    .enable_prometheus()
    .collection_interval(Duration::from_secs(15))
    .build();

let config = DiscoveryConfig::builder()
    .metrics_config(metrics_config)
    .build();
```

3. Production Safety Features
   - Rate limiting
   - Circuit breaking
   - Load balancing

```rust
let safety_config = SafetyConfig::builder()
    .rate_limit(100, Duration::from_secs(1))
    .circuit_breaker(CircuitBreakerConfig::default())
    .load_balancing(Strategy::LeastConnections)
    .build();
```

### Deprecated Features

1. Simple Configuration Struct
   - `DiscoveryConfig` struct constructor deprecated
   - Use builder pattern instead
   - Will be removed in 2.0.0

1. Global Functions
   - `set_global_timeout` removed
   - Use configuration builder instead

1. Direct Protocol Access
   - Direct protocol structs deprecated
   - Use ProtocolManager interface instead

## Migration Steps

1. Update Dependencies

   ```toml
   [dependencies]
   auto-discovery = "1.0"
   ```

1. Update Configuration
   - Replace direct struct usage with builders
   - Configure each protocol separately
   - Add security settings if needed

1. Update Error Handling
   - Use new error types
   - Add context where needed
   - Handle new error variants

1. Enable New Features
   - Add metrics configuration
   - Configure security features
   - Enable production safety features

1. Test Changes
   - Run integration tests
   - Check metrics
   - Verify security settings

## Common Issues

### Cache Configuration

Problem:
```rust
// Old: This will not compile
config.cache_size = 1000;
```

Solution:
```rust
// New: Use builder pattern
let config = DiscoveryConfig::builder()
    .cache_config(
        CacheConfig::builder()
            .max_entries(1000)
            .build()
    )
    .build();
```

### Protocol Selection

Problem:
```rust
// Old: This will not compile
config.enable_mdns();
```

Solution:
```rust
// New: Use protocol-specific config
let config = DiscoveryConfig::builder()
    .with_mdns(MdnsConfig::default())
    .build();
```

### Error Handling

Problem:
```rust
// Old: Missing context
match result {
    Err(DiscoveryError::Protocol(_)) => // Handle error
}
```

Solution:
```rust
// New: With context
match result {
    Err(DiscoveryError::Protocol { source, context }) => {
        error!("Protocol error: {}, Context: {}", source, context);
    }
}
```

## Compatibility Tools

The crate provides tools to help with migration:

1. Configuration Converter
   ```rust
   use auto_discovery::compat::ConfigConverter;
   
   let old_config = /* old config */;
   let new_config = ConfigConverter::to_new_config(old_config);
   ```

2. Error Converter
   ```rust
   use auto_discovery::compat::ErrorConverter;
   
   let old_error = /* old error */;
   let new_error = ErrorConverter::to_new_error(old_error);
   ```

## Getting Help

If you encounter issues during migration:

1. Check the [FAQ](./FAQ.md)
1. Open an issue on GitHub
1. Join our Discord community
1. Contact us at [support@auto-discovery.rs](mailto:support@auto-discovery.rs)
