<!-- Use this file to provide workspace-specific custom instructions to Copilot. For more details, visit https://code.visualstudio.com/docs/copilot/copilot-customization#_use-a-githubcopilotinstructionsmd-file -->

# Auto Discovery Crate Development Guidelines

This is a Rust library crate for network and system service discovery. The crate provides a general-purpose solution for automatically detecting, connecting to, and coordinating with other services in a network environment.

## Project Structure

- **src/lib.rs**: Main library entry point with public API exports
- **src/error.rs**: Error types and result handling
- **src/types.rs**: Core type definitions (ServiceType, ProtocolType, NetworkInterface, etc.)
- **src/config.rs**: Configuration types for discovery and registration
- **src/service.rs**: ServiceInfo and ServiceEvent definitions
- **src/discovery.rs**: Main ServiceDiscovery implementation
- **src/protocols/**: Protocol implementations (mDNS, UPnP, etc.)
- **src/utils.rs**: Utility functions for network, time, string manipulation, and validation
- **examples/**: Usage examples

## Key Design Principles

1. **Async-first**: All I/O operations use tokio and async/await
2. **Cross-platform**: Support Windows, Linux, and macOS
3. **Protocol-agnostic**: Extensible architecture for multiple discovery protocols
4. **Type-safe**: Leverage Rust's type system for correctness
5. **Error handling**: Comprehensive error types with detailed context
6. **Testable**: Mock implementations for unit testing

## Code Style Guidelines

- Use `tracing` for logging (debug, info, warn, error levels)
- Prefer `Result<T>` return types for fallible operations
- Use builder patterns for configuration types
- Include comprehensive documentation with examples
- Write unit tests for all public functions
- Use `#[cfg(test)]` modules for tests

## Protocol Implementation Notes

- **mDNS**: Currently has placeholder implementation - needs integration with actual mDNS crate
- **UPnP**: Mock SSDP discovery implementation - needs real network implementation
- **DNS-SD**: Not yet implemented
- **SSDP**: Not yet implemented

## Dependencies

- `tokio`: Async runtime and utilities
- `futures`: Stream traits and utilities
- `serde`: Serialization/deserialization
- `tracing`: Logging and diagnostics
- `thiserror`: Error handling
- `uuid`: Unique identifiers
- `async-trait`: Async trait support

## Testing Strategy

- Unit tests for individual modules
- Mock implementations for network protocols during testing
- Integration tests for end-to-end scenarios
- Platform-specific tests for network interface detection

## Future Extensions

- Real protocol implementations replacing mocks
- Additional discovery protocols (Consul, etcd, etc.)
- Service health monitoring
- Load balancing integration
- Security and authentication features
- Performance optimizations

## Error Handling Patterns

Use the custom `DiscoveryError` enum for all library errors. Provide meaningful error messages with context. Use `Result<T>` consistently and avoid panicking in library code.

## Memory Management

Be conscious of memory usage, especially for long-running discovery processes. Use `Arc` and `Mutex` judiciously for shared state. Prefer message passing over shared memory where possible.
