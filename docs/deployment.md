# Production Deployment Guide

This guide covers deploying the auto-discovery library in production environments.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Security Configuration](#security-configuration)
3. [Environment Configuration](#environment-configuration)
4. [Monitoring Setup](#monitoring-setup)
5. [Health Checks](#health-checks)
6. [Performance Tuning](#performance-tuning)
7. [Troubleshooting](#troubleshooting)

## Prerequisites

- Rust 1.75 or later
- Network access for service discovery protocols
- Proper DNS server configuration (for DNS-SD)
- Required permissions for dynamic DNS updates (if using)

## Security Configuration

### TSIG Authentication

For secure DNS updates, configure TSIG authentication:

```env
DISCOVERY_TSIG_KEYNAME=update-key.
DISCOVERY_TSIG_SECRET=your-base64-secret
```

### TLS Configuration

For secure communication:

```env
DISCOVERY_TLS_CERT=/path/to/cert.pem
DISCOVERY_TLS_KEY=/path/to/key.pem
```

### Rate Limiting

Configure rate limits to prevent abuse:

```env
DISCOVERY_RATE_LIMIT=1000  # Operations per second
DISCOVERY_BURST_LIMIT=100  # Burst size
```

## Environment Configuration

Basic configuration:

```env
DISCOVERY_DOMAIN=your.domain
DISCOVERY_TTL=3600
DISCOVERY_TIMEOUT=5
DISCOVERY_CACHE_SIZE=1000
DISCOVERY_CACHE_TTL=300
DISCOVERY_HEALTH_CHECK_INTERVAL=30
```

### DNS Server Configuration:

```env
DISCOVERY_DNS_SERVER=192.168.1.1:53
```

### Metrics Configuration:

```env
DISCOVERY_METRICS_ENABLED=true
DISCOVERY_METRICS_PORT=9000
```

## Monitoring Setup

### Prometheus Metrics

The library exports metrics on `http://localhost:9000/metrics` including:

- Discovery latencies
- Cache hit/miss rates
- Service counts
- Health check results
- Operation success/failure rates

### Grafana Dashboard

Import the provided Grafana dashboard template for visualization:

```json
{
    "dashboard": {
        "id": "auto-discovery",
        "title": "Auto Discovery Metrics",
        "panels": [...]
    }
}
```

## Health Checks

The library performs automatic health checks:

- TCP connection tests
- DNS resolution verification
- Service availability monitoring

Configure health check behavior:

```env
DISCOVERY_HEALTH_CHECK_INTERVAL=30
DISCOVERY_HEALTH_CHECK_TIMEOUT=5
```

## Performance Tuning

### Cache Configuration

Optimize cache settings based on your environment:

```env
DISCOVERY_CACHE_SIZE=5000        # Increase for busy environments
DISCOVERY_CACHE_TTL=600         # Adjust based on service stability
```

### Connection Pooling

Configure connection pools:

```env
DISCOVERY_DNS_POOL_SIZE=20
DISCOVERY_TCP_POOL_SIZE=50
```

### Memory Management

Monitor memory usage and adjust:

```env
DISCOVERY_MAX_SERVICES=10000    # Limit total tracked services
DISCOVERY_GC_INTERVAL=3600     # Memory cleanup interval
```

## Troubleshooting

### Common Issues

1. DNS Update Failures
   - Verify TSIG configuration
   - Check DNS server permissions
   - Validate network connectivity

2. Cache Performance
   - Monitor cache hit rates
   - Adjust cache size and TTL
   - Check memory usage

3. Service Discovery Timeouts
   - Verify network latency
   - Check rate limits
   - Adjust timeout values

### Logging

Enable detailed logging:

```env
RUST_LOG=debug
DISCOVERY_LOG_FILE=/var/log/discovery.log
```

### Metrics Analysis

Key metrics to monitor:

```prometheus
# Discovery performance
discovery_operation_duration_seconds
discovery_cache_hit_ratio

# Health status
discovery_health_check_success_ratio
discovery_service_availability_percent

# Resource usage
discovery_memory_usage_bytes
discovery_open_connections
```

## Support and Maintenance

### Backup and Recovery

1. Regularly backup configuration
2. Document custom settings
3. Maintain service inventory

### Updates and Migrations

1. Follow semantic versioning
2. Test in staging environment
3. Plan maintenance windows
4. Document breaking changes

### Emergency Procedures

1. Stop service discovery
2. Clear cache if corrupted
3. Reset to default configuration
4. Restore from backup

## Security Best Practices

1. Use TSIG authentication for DNS updates
2. Enable TLS for all connections
3. Implement rate limiting
4. Regular security audits
5. Keep dependencies updated
6. Monitor for suspicious patterns

## Further Reading

- [Rust Documentation](https://docs.rs/auto-discovery)
- [Protocol Specifications](./protocols.md)
- [API Reference](./api.md)
- [Security Guide](./security.md)
