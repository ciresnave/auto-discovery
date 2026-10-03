# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - Unreleased

Non-breaking. This release makes CI able to pass, and picks up patched versions
of transitive dependencies.

### Fixed

- **`--features simple-mdns` did not compile** (`E0433`). The `simple_mdns`
  module had been disabled, but the protocol manager still called it. As a
  result `--all-features`, and so CI's `cargo test --all-features`, could never
  build. With both `simple-mdns` and `mdns` enabled, neither mDNS branch ran,
  so mDNS silently disappeared. Every configuration now uses the working mdns-sd
  backend. The dead `simple-mdns` backend is removed in 0.3.0.
- **Three crate-level doc examples did not compile.** They called a nonexistent
  `DiscoveryConfig::add_service_type`; the real API is `with_service_type`. A
  stray code fence turned prose into code. The three examples that perform live
  network discovery are marked `no_run`: they are still compiled, but not
  executed.
- `clippy -D warnings`: six `collapsible_if` fixes.

### Security

- Refreshed `Cargo.lock` to the latest in-range versions. Advisory hits across
  the locked dependency graph (OSV, 2026-10-03) fell from **50 across 22 crates
  to 10 across 8**. That clears the patched advisories in `aws-lc-sys`,
  `bytes`, `crossbeam-epoch`, `event-listener`, `h2`, `openssl`, `rand`,
  `rustls`, `rustls-webpki`, `slab`, `time` and `tracing-subscriber`.
- **Remaining, and fixed in 0.3.0:**
  - `trust-dns` 0.23 (renamed to `hickory-dns`, RUSTSEC-2025-0017), which pulls
    in `idna` 0.4 (RUSTSEC-2024-0421);
  - `quick-xml` 0.38 (RUSTSEC-2026-0194/0195);
  - `backoff` and `instant` (unmaintained);
  - `async-std`, `net2` and `proc-macro-error`, all unmaintained and all pulled
    in by the optional `mdns` 3.0 dependency.

### Changed

- The whole crate is now `rustfmt`-formatted, which CI's `cargo fmt --check`
  requires. That is most of this diff, and is formatting only.

## [0.2.0] - 2025-07-15

### Added

- **Simple API Module** (`src/simple.rs`) - One-liner functions for easy usage
- **Service Registry** (`src/registry.rs`) - Centralized service management and discovery
- **Health Monitoring** (`src/health.rs`) - Comprehensive health checks and status tracking
- **Metrics Collection** (`src/metrics.rs`) - Performance monitoring and Prometheus integration
- **Load Balancing** (`src/safety/load_balancer.rs`) - Smart service selection strategies
- **Graceful Shutdown** (`src/shutdown.rs`) - Clean termination handling
- **Real UPnP/SSDP Implementation** - Working multicast discovery with actual network protocols
- **Multiple mDNS Backends** - Alternative implementations for different use cases
- **11 Working Examples** - Comprehensive usage demonstrations
- **62 Total Tests** - Extensive test coverage including real network tests

### Enhanced

- **Production Safety Features** - Rate limiting, circuit breakers, retry mechanisms
- **Security Features** - Feature-gated security with TLS/native-TLS support
- **Error Handling** - Improved error context and protocol-specific error types
- **Documentation** - Production-ready guides and API documentation
- **Cross-platform Support** - Enhanced Windows, Linux, and macOS compatibility

### Fixed

- **Windows Build Compatibility** - Replaced rustls with native-tls for broader compatibility
- **Feature Gates** - Proper conditional compilation for optional features
- **Dependencies** - Updated to latest versions with security fixes
- **Clippy Warnings** - Zero warnings, clean code quality

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
