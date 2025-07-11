# Security Best Practices

This document outlines security best practices for using the auto-discovery library in production environments.

## Authentication and Authorization

### TSIG Authentication

Always use TSIG authentication for DNS updates:

```rust
let config = DiscoveryConfig {
    tsig_keyname: Some("update-key.example.com."),
    tsig_secret: Some("your-base64-secret"),
    ..Default::default()
};
```

### TLS Configuration

Enable TLS for all network connections:

```rust
let config = DiscoveryConfig {
    tls_cert_path: Some("/path/to/cert.pem".into()),
    tls_key_path: Some("/path/to/key.pem".into()),
    ..Default::default()
};
```

## Rate Limiting

Implement rate limiting to prevent abuse:

```rust
let config = DiscoveryConfig {
    rate_limit: Some(1000),  // Operations per second
    burst_limit: Some(100),  // Burst size
    ..Default::default()
};
```

## Network Security

### Firewall Configuration

Required ports:
- DNS (53/udp, 53/tcp)
- mDNS (5353/udp)
- UPnP/SSDP (1900/udp)
- Metrics (9000/tcp)

### Network Isolation

Use network segmentation:
```plaintext
DMZ:
  - DNS servers
  - Load balancers
  - Metrics collectors

Internal:
  - Service discovery agents
  - Application services
```

## Monitoring and Alerting

### Security Metrics

Monitor security-related metrics:
```prometheus
# Authentication failures
discovery_auth_failures_total
discovery_tsig_verification_failures_total

# Rate limiting
discovery_rate_limit_exceeded_total
discovery_burst_limit_exceeded_total

# Network issues
discovery_connection_failures_total
discovery_dns_update_failures_total
```

### Alert Configuration

Set up alerts for:
- Authentication failures
- Rate limit breaches
- Certificate expiration
- Network connectivity issues

## Audit Logging

Enable comprehensive audit logging:

```rust
use tracing::{info, warn, error};

// Log security events
info!(
    event = "service_registered",
    service_id = %service.id,
    client_ip = %client_addr
);

warn!(
    event = "auth_failure",
    reason = "invalid_tsig",
    client_ip = %client_addr
);
```

## Data Protection

### Service Information

- Limit exposed information in service records
- Use TXT records judiciously
- Validate all service data

### Cache Security

- Implement cache entry validation
- Set appropriate TTLs
- Regular cache cleanup

## Secure Configuration

### Environment Variables

Use secure environment configuration:

```env
# Required
DISCOVERY_TSIG_KEYNAME=update-key.example.com.
DISCOVERY_TSIG_SECRET=<base64-secret>
DISCOVERY_TLS_CERT=/path/to/cert.pem
DISCOVERY_TLS_KEY=/path/to/key.pem

# Optional
DISCOVERY_RATE_LIMIT=1000
DISCOVERY_BURST_LIMIT=100
DISCOVERY_AUTH_TIMEOUT=5
```

### File Permissions

Set correct file permissions:
```bash
# Certificate files
chmod 600 /path/to/key.pem
chmod 644 /path/to/cert.pem

# Configuration files
chmod 600 /etc/discovery/config.yaml
```

## Security Updates

### Dependency Management

Keep dependencies updated:
```toml
[dependencies]
ring = "0.17"        # Cryptography
rustls = "0.21"      # TLS
x509-parser = "0.15" # Certificate handling
```

### Version Policy

- Follow semantic versioning
- Document security fixes
- Maintain changelog

## Incident Response

### Common Issues

1. Authentication Failures
   - Check TSIG configuration
   - Verify key synchronization
   - Review audit logs

2. Network Security
   - Monitor for unusual patterns
   - Check firewall rules
   - Verify TLS configuration

3. Rate Limiting
   - Adjust limits if needed
   - Investigate traffic spikes
   - Block malicious clients

### Recovery Procedures

1. Revoke compromised credentials
2. Rotate TSIG keys
3. Update TLS certificates
4. Clear and rebuild cache
5. Update firewall rules

## Compliance

### Audit Requirements

Maintain records of:
- Service registrations
- Authentication attempts
- Configuration changes
- Security incidents

### Data Retention

Configure retention policies:
```rust
let config = DiscoveryConfig {
    audit_log_retention_days: 90,
    metrics_retention_days: 30,
    ..Default::default()
};
```

## Testing

### Security Testing

Regular security checks:
- Penetration testing
- Configuration review
- Dependency scanning
- Network security audit

### Validation

Verify security measures:
```rust
#[test]
fn test_tsig_security() {
    // Test TSIG authentication
}

#[test]
fn test_tls_configuration() {
    // Test TLS setup
}

#[test]
fn test_rate_limiting() {
    // Test rate limits
}
```
