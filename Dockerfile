# Multi-stage Docker build for Hippo CLI
# Note: GUI (Tauri) app cannot run in Docker without X11/Wayland

# Build stage
FROM rust:1.85-slim-bookworm AS builder

# Cap lints to warnings for dependencies (required for zune-jpeg SIMD intrinsics)
ENV CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_OPT_LEVEL="2"
ENV RUSTFLAGS="--cap-lints warn"


# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy workspace members and manifests
COPY hippo-core ./hippo-core
COPY hippo-cli ./hippo-cli
COPY Cargo.lock ./

# Create minimal workspace manifest (excluding packages that aren't needed for CLI)
COPY Cargo.toml Cargo.toml.orig
RUN sed -e '/hippo-tauri/d' -e '/hippo-wasm/d' -e '/hippo-web/d' Cargo.toml.orig > Cargo.toml && rm Cargo.toml.orig

# Build release binary (CLI only - Tauri requires desktop environment, use existing Cargo.lock)
RUN cargo build --release --package hippo-cli

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    sqlite3 \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 hippo && \
    mkdir -p /data && \
    chown -R hippo:hippo /data

# Copy binary from builder
COPY --from=builder /app/target/release/hippo /usr/local/bin/hippo

# Set user
USER hippo

# Set working directory
WORKDIR /data

# Set environment variables
ENV HIPPO_DB_PATH=/data/hippo.db
ENV RUST_LOG=info

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD hippo --version || exit 1

# Default command
ENTRYPOINT ["/usr/local/bin/hippo"]
CMD ["--help"]
