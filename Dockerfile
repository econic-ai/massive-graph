# Multi-stage Dockerfile for Massive Graph Database
# Optimized for performance and minimal image size

# Build stage
FROM rust:1.87-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    make \
    gcc \
    libc6-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf src

# Copy source code
COPY src ./src
COPY benches ./benches

# Build the application with optimizations
RUN cargo build --release

# Development stage
FROM rust:1.87-slim-bookworm AS development

# Install debugging tools and dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    make \
    gcc \
    libc6-dev \
    gdb \
    gdbserver \
    lldb-14 \
    netcat-openbsd \
    lsof \
    procps \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch for development
RUN cargo install cargo-watch

WORKDIR /app

# Copy all project files to the development image
COPY . .

# Make scripts executable
RUN chmod +x .bin/*.sh

# Don't auto-run anything - scripts will be run manually or by compose/k8s

# Production stage
FROM debian:bookworm-slim AS production

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user for security
RUN useradd -r -s /bin/false -m -d /app massive-graph

# Create data directory
RUN mkdir -p /app/data && chown massive-graph:massive-graph /app/data

# Copy the binary and scripts from builder stage
COPY --from=builder /app/target/release/massive-graph /usr/local/bin/massive-graph
COPY .bin/prod.sh /app/.bin/prod.sh

# Set ownership
RUN chown massive-graph:massive-graph /usr/local/bin/massive-graph /app/.bin/prod.sh && \
    chmod +x /app/.bin/prod.sh

# Switch to app user
USER massive-graph
WORKDIR /app

# Expose ports
EXPOSE 8080 8081 8082 9090

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the production script
CMD ["bash", "/app/.bin/prod.sh"] 