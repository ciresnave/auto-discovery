# Build stage
FROM rust:1.75-slim-bookworm as builder

WORKDIR /usr/src/auto-discovery
COPY . .

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Build the project
RUN cargo build --release

# Test stage
FROM rust:1.75-slim-bookworm as test

WORKDIR /usr/src/auto-discovery
COPY --from=builder /usr/src/auto-discovery .

# Install test dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Run tests
RUN cargo test --release

# Production stage
FROM debian:bookworm-slim

# Create a non-root user
RUN useradd -m -u 1000 -s /bin/bash appuser

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Copy the built binary
COPY --from=builder /usr/src/auto-discovery/target/release/auto-discovery /usr/local/bin/
COPY --from=builder /usr/src/auto-discovery/config /etc/auto-discovery/config

# Set up configuration directory
RUN mkdir -p /etc/auto-discovery && \
    chown -R appuser:appuser /etc/auto-discovery

# Switch to non-root user
USER appuser

# Set environment variables
ENV RUST_LOG=info
ENV CONFIG_PATH=/etc/auto-discovery/config/default.yaml

# Expose ports for mDNS, SSDP/UPnP, metrics, and health check
EXPOSE 5353/udp
EXPOSE 1900/udp
EXPOSE 9090
EXPOSE 8080

CMD ["auto-discovery"]
