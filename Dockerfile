# Multi-stage Docker build for Hippo CLI
# Note: GUI (Tauri) app cannot run in Docker without X11/Wayland

# Build stage
FROM rust:1.83-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy lock file first
COPY Cargo.lock ./

# Copy workspace members
COPY hippo-core ./hippo-core
COPY hippo-cli ./hippo-cli

# Create Docker-specific workspace manifest (without hippo-tauri)
RUN cat > Cargo.toml << 'EOF'
[workspace]
resolver = "2"
members = [
    "hippo-core",
    "hippo-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Hippo Contributors"]
license = "MIT"
repository = "https://github.com/user/hippo"

[workspace.dependencies]
tokio = { version = "1.40", features = ["full"] }
async-trait = "0.1"
futures = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
qdrant-client = "1.12"
ort = { version = "2.0.0-rc.10", features = ["download-binaries"] }
ndarray = "0.16"
walkdir = "2.5"
mime_guess = "2.0"
image = "0.25"
symphonia = "0.5"
pdf-extract = "0.7"
zip = "2.2"
kamadak-exif = "0.5"
tree-sitter = "0.24"
tree-sitter-rust = "0.23"
tree-sitter-python = "0.23"
tree-sitter-javascript = "0.23"
rusqlite = { version = "0.32", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.10", features = ["v4", "serde"] }
thiserror = "2.0"
anyhow = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
directories = "5.0"
notify = "7.0"
rayon = "1.10"
clap = { version = "4.5", features = ["derive", "color", "env"] }
colored = "2.1"
indicatif = "0.17"
dialoguer = "0.11"
tabled = "0.16"
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }
sha2 = "0.10"
ctrlc = "3.4"
lru = "0.12"
parking_lot = "0.12"
flate2 = "1.0"
tar = "0.4"
EOF

# Build release binary (CLI only - Tauri requires desktop environment)
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
