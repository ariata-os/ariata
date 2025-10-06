# Multi-stage Dockerfile for Ariata Rust Core
# Stage 1: Build the Rust binary
FROM rust:1.86-slim as builder

# Install dependencies for compilation
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./

# Copy source code
COPY src ./src
COPY migrations ./migrations

# Build release binary
RUN cargo build --release --bin ariata

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/ariata /usr/local/bin/ariata

# Copy migrations for runtime execution
COPY --from=builder /app/migrations /app/migrations

# Expose HTTP server port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV DATABASE_URL=""
ENV MINIO_ENDPOINT=""

# Run migrations on startup, then start the server
# Configuration via environment variables (HOST, PORT, DATABASE_URL, etc)
CMD ["ariata"]
