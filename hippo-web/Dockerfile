# Multi-stage build for Hippo Web API
# Build stage
FROM rust:1.75 as builder

WORKDIR /app

# Copy workspace manifests
COPY Cargo.toml ./
COPY hippo-core ./hippo-core
COPY hippo-web ./hippo-web

# Build the application
WORKDIR /app/hippo-web
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 hippo

# Copy binary from builder
COPY --from=builder /app/target/release/hippo-web /usr/local/bin/hippo-web

# Create data directory
RUN mkdir -p /data && chown hippo:hippo /data

# Switch to app user
USER hippo

# Set environment variables
ENV HIPPO_HOST=0.0.0.0
ENV HIPPO_PORT=3000
ENV HIPPO_DATA_DIR=/data

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/api/health || exit 1

# Run the application
CMD ["hippo-web"]
