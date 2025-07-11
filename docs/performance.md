# Performance Tuning Guide

This guide provides detailed information on optimizing the auto-discovery library for maximum performance in production environments.

## Table of Contents

1. [Cache Optimization](#cache-optimization)
2. [Network Configuration](#network-configuration)
3. [Resource Management](#resource-management)
4. [Protocol Selection](#protocol-selection)
5. [Monitoring and Metrics](#monitoring-and-metrics)
6. [Load Testing](#load-testing)
7. [Security Impact](#security-impact)
8. [Platform-Specific Tuning](#platform-specific-tuning)

## Cache Optimization

### Cache Configuration

Configure cache size and behavior using the builder pattern:

```rust
use auto_discovery::config::{CacheConfig, DiscoveryConfig};
use std::time::Duration;

let cache_config = CacheConfig::builder()
    .max_entries(10_000)               // Maximum services to cache
    .ttl(Duration::from_secs(600))     // Cache TTL (10 minutes)
    .negative_ttl(Duration::from_secs(60))  // Negative cache TTL
    .prefetch_threshold(0.8)           // Refresh at 80% TTL
    .build();

let config = DiscoveryConfig::builder()
    .cache_config(cache_config)
    .build();
```

### Cache Strategies

Choose the right caching strategy based on your environment:

#### High-Traffic Production

```rust
let cache_config = CacheConfig::builder()
    .max_entries(50_000)
    .ttl(Duration::from_secs(1800))    // 30 minutes
    .prefetch_threshold(0.8)
    .enable_compression(true)           // Compress cache entries
    .persistence_path(Some("/var/cache/autodiscovery"))
    .build();
```

#### Development/Testing

```rust
let cache_config = CacheConfig::builder()
    .max_entries(1_000)
    .ttl(Duration::from_secs(300))     // 5 minutes
    .prefetch_threshold(0.9)
    .enable_compression(false)
    .build();
```

## Network Configuration

### Connection Management

```rust
use auto_discovery::config::NetworkConfig;

let network_config = NetworkConfig::builder()
    .dns_pool_size(20)
    .tcp_pool_size(50)
    .udp_buffer_size(8192)
    .tcp_keepalive(Duration::from_secs(60))
    .tcp_nodelay(true)
    .build();
```

### Protocol-Specific Tuning

#### DNS-SD

```rust
let dns_config = DnsConfig::builder()
    .timeout(Duration::from_secs(5))
    .retry_count(2)
    .tcp_fallback(true)                // Use TCP on UDP failure
    .edns0_enabled(true)               // Enable EDNS0
    .build();
```

#### mDNS

```rust
let mdns_config = MdnsConfig::builder()
    .query_interval(Duration::from_secs(10))
    .ttl(Duration::from_secs(60))
    .multicast_ttl(4)                  // Multicast TTL
    .build();
```

## Resource Management

### Memory Control

```rust
use auto_discovery::config::{ResourceConfig, MemoryConfig};

let memory_config = MemoryConfig::builder()
    .max_services(10_000)
    .max_concurrent_operations(100)
    .gc_interval(Duration::from_secs(3600))
    .build();

let resource_config = ResourceConfig::builder()
    .memory_config(memory_config)
    .build();
```

### Runtime Configuration

```rust
use tokio::runtime;

let runtime = runtime::Builder::new_multi_thread()
    .worker_threads(4)
    .max_blocking_threads(10)
    .thread_name("autodiscovery-worker")
    .enable_all()
    .build()
    .unwrap();
```

## Monitoring and Metrics

### Prometheus Integration

```rust
use auto_discovery::metrics::{MetricsConfig, PrometheusExporter};

let metrics_config = MetricsConfig::builder()
    .enable_prometheus()
    .collection_interval(Duration::from_secs(15))
    .build();

// Example metrics
// autodiscovery_services_total{protocol="mdns"} 42
// autodiscovery_cache_hit_ratio{} 0.95
// autodiscovery_discovery_duration_seconds{} 0.123
```

### Health Checks

```rust
use auto_discovery::health::{HealthConfig, HealthCheck};

let health_config = HealthConfig::builder()
    .check_interval(Duration::from_secs(30))
    .failure_threshold(3)
    .build();
```

## Benchmark Results

| Scenario | Services | Memory (MB) | CPU (%) | Network (KB/s) | Cache Hit % |
|----------|----------|-------------|---------|----------------|-------------|
| Light    | 100      | 20         | 1-2     | 5-10          | 95%         |
| Medium   | 1,000    | 50         | 5-10    | 20-50         | 90%         |
| Heavy    | 10,000   | 200        | 15-25   | 100-200       | 85%         |

### Protocol Performance Comparison

| Protocol | Discovery Time | CPU Usage | Network Usage | Scalability |
|----------|---------------|-----------|---------------|-------------|
| DNS-SD   | 500ms        | Low       | Low           | High        |
| mDNS     | 100ms        | Low       | Medium        | Medium      |
| UPnP     | 200ms        | Medium    | High          | Medium      |

## Security Impact on Performance

Security features may impact performance:

| Feature              | CPU Impact | Memory Impact | Network Impact |
|---------------------|------------|---------------|----------------|
| TSIG Authentication | +2-5%      | Negligible    | +5%           |
| TLS/DTLS           | +10-15%    | +10MB         | +20%          |
| Certificate Pinning | +1-2%      | Negligible    | None          |

## Platform-Specific Tuning

### Windows Optimization

```rust
let config = DiscoveryConfig::builder()
    .windows_specific()
    .use_iocp(true)                     // Use I/O Completion Ports
    .tcp_loopback_fast_path(true)       // Fast loopback
    .build();
```

### Linux Optimization

```rust
let config = DiscoveryConfig::builder()
    .linux_specific()
    .use_epoll(true)                    // Use epoll
    .reuse_port(true)                   // Enable SO_REUSEPORT
    .build();
```

## Production Checklist

1. Cache Configuration
   - [ ] Set appropriate cache size
   - [ ] Configure TTLs
   - [ ] Enable prefetching
   - [ ] Consider persistence

2. Network Settings
   - [ ] Configure connection pools
   - [ ] Set appropriate timeouts
   - [ ] Enable TCP keepalive
   - [ ] Configure buffer sizes

3. Resource Management
   - [ ] Set memory limits
   - [ ] Configure thread pools
   - [ ] Enable garbage collection
   - [ ] Set operation limits

4. Monitoring
   - [ ] Enable Prometheus metrics
   - [ ] Configure health checks
   - [ ] Set up alerts
   - [ ] Enable tracing
