# Protocol Compatibility Matrix

This document outlines the compatibility and feature support for each discovery protocol in the auto-discovery crate.

## Protocol Overview

| Feature | DNS-SD | mDNS | UPnP/SSDP |
|---------|--------|------|-----------|
| Version | 1.1 | 1.0 | 1.1 |
| Status | Production | Production | Production |
| Transport | TCP/UDP | UDP | UDP |
| IPv4 | ✅ | ✅ | ✅ |
| IPv6 | ✅ | ✅ | ⚠️ Limited |
| Secure Updates | ✅ TSIG+DNSSEC | ❌ | ❌ |
| TLS Support | ✅ | ❌ | ✅ |

## Feature Support

| Feature | DNS-SD | mDNS | UPnP/SSDP |
|---------|--------|------|-----------|
| Service Registration | ✅ | ✅ | ✅ |
| Service Discovery | ✅ | ✅ | ✅ |
| Service Updates | ✅ | ✅ | ✅ |
| Service Removal | ✅ | ✅ | ✅ |
| TXT Records | ✅ | ✅ | ✅ |
| Health Monitoring | ✅ | ✅ | ✅ |
| Metrics Export | ✅ Prometheus | ✅ Prometheus | ✅ Prometheus |
| OpenTelemetry | ✅ | ✅ | ✅ |
| Rate Limiting | ✅ | ✅ | ✅ |
| Circuit Breaking | ✅ | ✅ | ✅ |
| Load Balancing | ✅ | ✅ | ✅ |
| Caching | ✅ | ✅ | ✅ |

## Security Features

| Feature | DNS-SD | mDNS | UPnP/SSDP |
|---------|--------|------|-----------|
| Authentication | ✅ TSIG+Kerberos | ❌ | ⚠️ Basic |
| Encryption | ✅ TLS 1.3 | ❌ | ✅ TLS 1.3 |
| Certificate Pinning | ✅ | ❌ | ✅ |
| Access Control | ✅ | ❌ | ⚠️ Limited |
| Update Security | ✅ TSIG | ❌ | ⚠️ Limited |
| Key Management | ✅ Vault/KMS | ❌ | ⚠️ Manual |
| Audit Logging | ✅ | ✅ | ✅ |

## Performance Characteristics

| Metric | DNS-SD | mDNS | UPnP/SSDP |
|--------|--------|------|-----------|
| Discovery Time | Medium (~500ms) | Fast (~100ms) | Fast (~200ms) |
| Network Usage | Low (<1KB/query) | Medium (~2KB/query) | Medium (~3KB/query) |
| Cache Efficiency | High (>90%) | Medium (~70%) | Medium (~60%) |
| Scalability | High (10k+ services) | Medium (<1k services) | Medium (<1k services) |
| CPU Usage | Low (<2%) | Low (<1%) | Low (<2%) |
| Memory Usage | Low (<50MB) | Low (<20MB) | Low (<30MB) |

## Platform Support

| Platform | DNS-SD | mDNS | UPnP/SSDP |
|----------|--------|------|-----------|
| Windows | ✅ | ✅ | ✅ |
| Linux | ✅ | ✅ | ✅ |
| macOS | ✅ | ✅ | ✅ |
| BSD | ✅ | ✅ | ✅ |
| Android | ⚠️ Limited | ✅ | ✅ |
| iOS | ⚠️ Limited | ✅ | ✅ |

## Network Requirements

| Requirement | DNS-SD | mDNS | UPnP/SSDP |
|------------|--------|------|-----------|
| Ports (TCP) | 53 | N/A | 80, 443 |
| Ports (UDP) | 53 | 5353 | 1900 |
| Multicast | Optional | Required | Required |
| DNS Server | Required | No | No |
| Firewall Config | TCP/UDP 53 | UDP 5353 | UDP 1900 |

## Error Handling

| Error Type | DNS-SD | mDNS | UPnP/SSDP |
|------------|--------|------|-----------|
| Timeout Recovery | ✅ | ✅ | ✅ |
| Retry Support | ✅ | ✅ | ✅ |
| Fallback Options | ✅ | ✅ | ✅ |
| Error Context | ✅ | ✅ | ✅ |
| Metrics | ✅ | ✅ | ✅ |

## Usage Recommendations

## Deployment Recommendations

### DNS-SD Deployment

- Best for enterprise environments
- Recommended when DNS infrastructure exists
- Suitable for secure service updates
- Preferred for large-scale deployments

### mDNS Deployment

- Best for local network discovery
- Recommended for zero-configuration setups
- Suitable for small to medium networks
- Preferred for rapid discovery needs

### UPnP/SSDP Deployment

- Best for device discovery
- Recommended for home/office networks
- Suitable for media devices

## Implementation Examples

### DNS-SD Implementation

```rust
use auto_discovery::{
    config::{DiscoveryConfig, DnsConfig},
    security::TsigKeyConfig,
};

let dns_config = DnsConfig::builder()
    .domain("example.com")
    .update_server("dns1.example.com")
    .tsig_key(TsigKeyConfig::new(
        "update-key",
        "HMAC-SHA256",
        "base64-encoded-key",
    ))
    .build();
```

### mDNS Implementation

```rust
use auto_discovery::config::{DiscoveryConfig, MdnsConfig};
use std::time::Duration;

let mdns_config = MdnsConfig::builder()
    .interface("eth0")
    .ttl(Duration::from_secs(60))
    .query_interval(Duration::from_secs(10))
    .build();
```

### UPnP Implementation

```rust
use auto_discovery::config::{DiscoveryConfig, UpnpConfig};

let upnp_config = UpnpConfig::builder()
    .search_target("urn:schemas-upnp-org:device:MediaServer:1")
    .mx(3)
    .interface("eth0")
    .build();
```

## Protocol Interoperability

The following describes how different protocols can work together in a mixed environment:

1. DNS-SD ↔ mDNS: Preserve service types and TXT records
- Preferred for consumer applications

## Configuration Examples

### DNS-SD Configuration
```rust
let config = DiscoveryConfig {
    protocol: ProtocolType::DnsSd,
    domain: Some("example.com".to_string()),
    dns_server: Some("192.168.1.1:53".parse()?),
    tsig_keyname: Some("update-key."),
    tsig_secret: Some("base64-secret"),
    ..Default::default()
};
```

### mDNS Configuration
```rust
let config = DiscoveryConfig {
    protocol: ProtocolType::Mdns,
    domain: Some("local".to_string()),
    interface: Some("eth0".to_string()),
    ..Default::default()
};
```

### UPnP Configuration
```rust
let config = DiscoveryConfig {
    protocol: ProtocolType::Upnp,
    interface: Some("eth0".to_string()),
    tls_enabled: true,
    ..Default::default()
};
```

## Migration Notes

When switching between protocols:
1. DNS-SD ↔ mDNS: Preserve service types and TXT records
2. UPnP ↔ DNS-SD: Map device types to service types
3. mDNS ↔ UPnP: Additional configuration may be needed

## Troubleshooting

| Issue | DNS-SD | mDNS | UPnP/SSDP |
|-------|--------|------|-----------|
| No Services Found | Check DNS server | Check multicast | Check SSDP traffic |
| Timeout | Verify DNS access | Check UDP 5353 | Check UDP 1900 |
| Auth Failure | Verify TSIG key | N/A | Check credentials |
| Network Issues | DNS resolution | Multicast routing | Multicast routing |
