# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-01-15

### Added

- Initial release with core functionality
- Async-first service discovery implementation
- Protocol implementations:
  - mDNS using `mdns-sd`
  - DNS-SD with TSIG support
  - UPnP/SSDP support
- Production safety features:
  - Rate limiting
  - Retry mechanisms
  - Health monitoring
  - Load balancing
  - Graceful shutdown
- Security features:
  - TSIG authentication
  - Multiple hash algorithms
  - Key rotation support
  - Secure update validation
- Monitoring and metrics:
  - Prometheus metrics
  - Health check endpoint
  - OpenTelemetry integration
  - Performance tracking
- Configuration management:
  - Environment variable support
  - YAML configuration
  - Builder pattern interface
- Comprehensive testing:
  - Unit tests
  - Integration tests
  - Platform-specific tests
  - Load/stress tests
  - Real network tests
- Documentation:
  - API documentation
  - Usage examples
  - Production deployment guide
  - Security best practices
  - Performance tuning guide
  - Protocol compatibility matrix

### Changed

- N/A (initial release)

### Deprecated

- N/A (initial release)

### Removed

- N/A (initial release)

### Fixed

- N/A (initial release)

### Security

- Initial security features implementation
- Comprehensive security documentation
- TSIG authentication support
- Secure update validation
