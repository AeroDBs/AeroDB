# AeroDB Dockerfile
# Multi-stage build for optimized image size

# Stage 1: Builder
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY benches ./benches

# Build release binary
RUN cargo build --release --bin aerodb

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create aerodb user
RUN useradd -m -u 1000 -s /bin/bash aerodb

# Create directories
RUN mkdir -p /var/lib/aerodb /etc/aerodb /var/log/aerodb && \
    chown -R aerodb:aerodb /var/lib/aerodb /var/log/aerodb

# Copy binary from builder
COPY --from=builder /build/target/release/aerodb /usr/local/bin/aerodb

# Switch to aerodb user
USER aerodb

# Set working directory
WORKDIR /var/lib/aerodb

# Expose port
EXPOSE 54321

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/aerodb", "--help"]

# Default command
ENTRYPOINT ["/usr/local/bin/aerodb"]
CMD ["start", "--config", "/etc/aerodb/config.json"]
