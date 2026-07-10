# Multi-stage build for Kojacoord Proxy
# NOTE: keep this Rust version at or above the highest MSRV among our
# transitive dependencies (e.g. time 0.3.47 needs 1.88, icu_provider
# needs 1.86). The release CI builds binaries with the latest stable
# toolchain, so the Docker builder must track it — pinning an old image
# here is what broke the multi-arch image build (rustc 1.85 < 1.88).
FROM rust:1.92-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    gcc \
    g++ \
    make \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY src ./src

# Build in release mode
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 kojacoord

# Create directories
RUN mkdir -p /app/plugins /app/logs \
    && chown -R kojacoord:kojacoord /app

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/kojacoord-proxy /app/kojacoord-proxy

# Switch to non-root user
USER kojacoord

# Expose the Minecraft proxy port and the server-management control-plane
# port. The proxy holds no persistent state and exposes no inbound
# HTTP/REST surface by design — there is no admin API port to expose here.
EXPOSE 25565 8080

# Health check: a raw TCP-connect against the Minecraft port. There's no
# HTTP endpoint in the container to curl (see above), so this is the only
# universally-available liveness signal; relies on bash's /dev/tcp, which
# ships in this base image.
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD bash -c 'echo > /dev/tcp/127.0.0.1/25565' || exit 1

# Run the proxy
CMD ["/app/kojacoord-proxy"]
