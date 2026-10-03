# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.3] - Unreleased

### Changed (CI only)

- **Codecov upload uses `codecov/codecov-action@v5`** (was v3). With the
  same `CODECOV_TOKEN`, the v3 uploader got `404 Repository not found` while
  v5 uploads succeed in another repo. The token and `fail_ci_if_error` are
  unchanged.

## [0.3.2] - Unreleased

### Added

- **`rust-version = "1.89"`** is now declared. It's the highest `rust-version`
  among the locked dependencies (`uuid` 1.27); the crate's own code needs 1.88
  for let-chains. A CI `msrv` job builds on 1.89. Some dependencies declare no
  `rust-version`, so that job, not the metadata, is the proof.
- **Test-only `network-tests` feature.** A plain `cargo test` no longer opens
  any mDNS/SSDP sockets. Each rebuilt test binary is a new program to the OS
  firewall, so every rebuild raised a fresh prompt.
  - The 14 lib tests in `discovery`, `protocols::*` and `simple` that build a
    discovery stack are `#[ignore]`d without the feature.
  - `tests/mdns_tests.rs` and `tests/upnp_tests.rs` aren't built without it
    (`required-features`).
  - The five doctests that construct a discovery stack are now `no_run`: they
    still compile but no longer execute.
  - CI enables the feature through `--all-features`. On a LAN, run the
    multicast tests with `cargo test --features network-tests -- --ignored`.

### Changed (CI only)

- **Every CI job has a fixed name**, so branch protection can require them:
  `fmt`, `clippy` (default, all and no features), `unit (linux, all
  features)`, `unit (windows, default features)`, `unit (macos, default
  features)`, `network (linux, all features)`, `no default features`,
  `msrv (1.89)`, `Security Audit`, `Benchmark`, `Code Coverage`.
- **Network tests are split from unit tests.**
  - The unit jobs run library tests that never touch the network.
  - The network job runs the rest: lib modules `discovery::tests`,
    `protocols::*` and `simple::tests`, the integration tests in `tests/`, and
    the doctests.
  - The two sets partition all 47 library tests exactly (31 + 16, no
    overlap).
- Every step uses `--locked`. The Codecov upload passes `CODECOV_TOKEN` once
  that secret exists.

## [0.3.1] - Unreleased

Bug fixes and dependency updates. Non-breaking for code that was correct; see the `name` note under Fixed.

### Changed: dependencies brought current

- **`mdns-sd` 0.13 → 0.21.** Since 0.21, `ServiceEvent::ServiceResolved`
  carries a `ResolvedService` (public fields) instead of a `ServiceInfo`; the
  converter is ported to it. **`From<mdns_sd::Error>` now refers to 0.21's error
  type.**
- **`base64` 0.22 → 0.23.** Only `Engine`, `STANDARD` and `DecodeError` are
  used, and their behaviour is unchanged.
- **`criterion` 0.6 → 0.8** (dev): the benches build with no warnings.
- **`rand` removed** from both dependencies and dev-dependencies: it was never
  used. The one `rand::` path in the code is `ring::rand::SystemRandom`.
- The declared minimums of the in-range dependencies are raised to their
  current versions.

### Fixed: mDNS discovery results (visible now that resolution works)

- **A discovered service's `name` is its instance name**, e.g. `my-svc` from
  `my-svc._http._tcp.local.`. It used to be the **hostname**
  (`my-host.local.`). That was a pre-existing bug: no `ServiceResolved` event
  reached the converter in the tests until mdns-sd 0.21.
- **Results no longer contain duplicates.** `discover_services` merges services
  found on the network with locally registered ones. It de-duplicated by `id`,
  but a network copy gets a fresh UUID, so the check never matched it and each
  service was returned twice. It now compares instance identity: name, type
  (up to a trailing `.local.`) and port. Repeated `ServiceResolved` events for
  one instance are collapsed the same way.
- **Test isolation:** each mDNS test now uses its own service type. With real
  resolution working, concurrently running tests that shared `_test._tcp`
  discovered each other's services.

## [0.3.0] - Unreleased

**Breaking.** Every advisory is cleared: OSV reports **0 hits** over the locked
dependency graph, down from 50 across 22 crates at 0.2.0. The crate is also now
what its documentation says it is.

### Security

- **trust-dns removed, not migrated.** Its only use was as the *type* of a dead
  private field (`DnsSdProtocol::client`). That struct is never constructed:
  its only constructor always returns an error. Removing the field and the
  dependency clears RUSTSEC-2025-0017 (trust-dns, renamed to hickory-dns) and
  RUSTSEC-2024-0421 (`idna` 0.4), with no new dependency. **DNS-SD remains
  unimplemented.** The `dns-sd` feature compiles a stub, whose constructor
  returns `DiscoveryError::Protocol("DNS-SD protocol not yet implemented")`.
- **`quick-xml` removed**, which clears RUSTSEC-2026-0194 and 0195. It was never
  imported.
- **Unmaintained crates removed:** `backoff` and `instant`, plus `async-std`,
  `net2` and `proc-macro-error`, which arrived through the external `mdns` 3.0
  crate. That crate was never used; the `mdns` module uses mdns-sd.

### Removed (breaking)

- **Features `mdns`, `simple-mdns`, `basic-mdns`, `metrics` and `testing`.**
  They enabled only dependencies that nothing compiled used, or a backend that
  never built. `secure` now enables only `ring`, and `upnp` enables no
  dependencies.
- **31 dependencies removed** (counted by diffing `Cargo.toml` against
  0.2.1). Each was found by a census of the crate, its tests, examples and
  benches, then confirmed by removal and build:
  - **26 normal:** `backoff`, `bytes`, `flume`, `futures`, `hex`, `hyper`,
    `mdns`, `metrics`, `metrics-exporter-prometheus`, `native-tls`,
    `quick-xml`, `reqwest`, `simple-mdns`, `socket2`, `tempfile`, `thiserror`,
    `tokio-metrics`, `tokio-stream`, `tokio-util`, `tower`, `tower-http`,
    `trust-dns-client` (see Security), `trust-dns-proto`,
    `trust-dns-resolver`, `url`, `x509-parser`;
  - **5 dev:** `mockall`, `proptest`, `tempfile`, `test-log`, `tokio-test`.

  None were added.

  The locked graph went from 399 packages to 158.
- **12 source files that were never compiled**, because no module declared
  them: `health`, `metrics`, `safety` (and `safety/load_balancer`), `shutdown`,
  `testing/stress`, `security/tsig`, and
  `protocols/{basic_mdns,libmdns,mdns_alt,simple_mdns,zeroconf}`. **The 0.2.0
  entry below claimed several of these as shipped features; it is corrected in
  place.**

### Fixed

- **`--no-default-features` now builds.** Each protocol module, its protocol
  manager initializer and the `mdns_sd` error conversion are gated on their
  feature (`dns-sd`, `mdns-sd`, `upnp`). Default, `--all-features`,
  `--no-default-features` and each backend alone all build, and `clippy
  --all-targets -D warnings` is clean in all of them.
- **The README and `docs/performance.md`** no longer document APIs from the
  never-compiled files (`safety`, `metrics`, `health`).

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
- ~~**Health Monitoring** (`src/health.rs`)~~ **Never shipped:** no module declared it, so it was never part of the built crate. Removed in 0.3.0.
- ~~**Metrics Collection** (`src/metrics.rs`)~~ **Never shipped:** no module declared it, so it was never part of the built crate. Removed in 0.3.0.
- ~~**Load Balancing** (`src/safety/load_balancer.rs`)~~ **Never shipped:** no module declared it, so it was never part of the built crate. Removed in 0.3.0.
- ~~**Graceful Shutdown** (`src/shutdown.rs`)~~ **Never shipped:** no module declared it, so it was never part of the built crate. Removed in 0.3.0.
- **Real UPnP/SSDP Implementation** - Working multicast discovery with actual network protocols
- ~~**Multiple mDNS Backends**~~ **Only the mdns-sd backend ever built.** The `simple-mdns` backend did not compile, and the others were undeclared files. Removed in 0.3.0.
- **11 Working Examples** - Comprehensive usage demonstrations
- **62 Total Tests** - Extensive test coverage including real network tests

### Enhanced

- ~~**Production Safety Features**~~ **Never shipped:** the rate-limiting, circuit-breaker and retry code lived in `src/safety.rs`, which no module declared it, so it was never part of the built crate. Removed in 0.3.0.
- **Security Features** - Feature-gated security with TLS/native-TLS support
- **Error Handling** - Improved error context and protocol-specific error types
- **Documentation** - Production-ready guides and API documentation
- **Cross-platform Support** - Enhanced Windows, Linux, and macOS compatibility

### Fixed

- **Windows Build Compatibility** - Replaced rustls with native-tls for broader compatibility
- ~~**Feature Gates**~~ **Not true at 0.2.0:** `--features simple-mdns` and `--no-default-features` did not compile. Fixed in 0.2.1 and 0.3.0.
- **Dependencies** - Updated to latest versions with security fixes
- ~~**Clippy Warnings**~~ **Not true at 0.2.0:** `clippy -D warnings` reported 6 errors. Fixed in 0.2.1.

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
