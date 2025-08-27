#!/bin/bash
# Production launch script for Massive Graph Database

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory (skip in container)
if [[ ! -f "Cargo.toml" && ! -f "/usr/local/bin/massive-graph" ]]; then
    print_error "Must be run from the massive-graph directory or in container"
    exit 1
fi

# Production environment variables with performance optimizations
export RUST_LOG=info
export MG_LOG_LEVEL=info
export MG_DATA_DIR="${MG_DATA_DIR:-/var/lib/massive-graph}"
export MG_HTTP_ADDR="${MG_HTTP_ADDR:-0.0.0.0:8080}"
export MG_WS_ADDR="${MG_WS_ADDR:-0.0.0.0:8081}"
export MG_QUIC_ADDR="${MG_QUIC_ADDR:-0.0.0.0:8082}"
export MG_WORKER_THREADS="${MG_WORKER_THREADS:-0}"  # Auto-detect
export MG_MAX_MEMORY="${MG_MAX_MEMORY:-8589934592}"  # 8GB default
export MG_MAX_CONNECTIONS="${MG_MAX_CONNECTIONS:-10000}"

# Performance tuning
export MALLOC_ARENA_MAX=4
export MALLOC_MMAP_THRESHOLD_=131072
export MALLOC_TRIM_THRESHOLD_=131072
export MALLOC_TOP_PAD_=131072
export MALLOC_MMAP_MAX_=65536

print_status "Starting Massive Graph Database in production mode..."
print_status "Configuration:"
echo "  - HTTP Server: $MG_HTTP_ADDR"
echo "  - WebSocket Server: $MG_WS_ADDR"
echo "  - QUIC Server: $MG_QUIC_ADDR"
echo "  - Data Directory: $MG_DATA_DIR"
echo "  - Worker Threads: $MG_WORKER_THREADS (0 = auto-detect)"
echo "  - Max Memory: $MG_MAX_MEMORY bytes"
echo "  - Max Connections: $MG_MAX_CONNECTIONS"
echo "  - Log Level: $MG_LOG_LEVEL"

# System checks
print_status "Performing system checks..."

# Check if running as root (not recommended for production)
if [[ $EUID -eq 0 ]]; then
    print_warning "Running as root is not recommended for production"
fi

# Check available memory
AVAILABLE_MEMORY=$(free -b | awk '/^Mem:/{print $7}')
if [[ $MG_MAX_MEMORY -gt $AVAILABLE_MEMORY ]]; then
    print_warning "Configured max memory ($MG_MAX_MEMORY) exceeds available memory ($AVAILABLE_MEMORY)"
fi

# Check CPU count
CPU_COUNT=$(nproc)
print_status "Detected $CPU_COUNT CPU cores"

# Create data directory if it doesn't exist
if [[ ! -d "$MG_DATA_DIR" ]]; then
    print_status "Creating data directory: $MG_DATA_DIR"
    mkdir -p "$MG_DATA_DIR"
fi

# Check data directory permissions
if [[ ! -w "$MG_DATA_DIR" ]]; then
    print_error "Data directory $MG_DATA_DIR is not writable"
    exit 1
fi

# Check if ports are available
check_port() {
    local addr=$1
    local name=$2
    local host=$(echo $addr | cut -d':' -f1)
    local port=$(echo $addr | cut -d':' -f2)
    
    if [[ "$host" == "0.0.0.0" ]]; then
        # Check if any process is listening on this port
        if ss -tuln | grep -q ":$port "; then
            print_error "$name port $port is already in use"
            return 1
        fi
    else
        # Check specific host:port combination
        if ss -tuln | grep -q "$addr "; then
            print_error "$name address $addr is already in use"
            return 1
        fi
    fi
}

print_status "Checking port availability..."
check_port $MG_HTTP_ADDR "HTTP" || exit 1
check_port $MG_WS_ADDR "WebSocket" || exit 1
check_port $MG_QUIC_ADDR "QUIC" || exit 1

print_success "All ports are available"

# Build in release mode for maximum performance (skip in container)
if [[ -f "Cargo.toml" ]]; then
    print_status "Building in release mode with optimizations..."
    if ! cargo build --release; then
        print_error "Build failed"
        exit 1
    fi
    print_success "Build completed"
else
    print_status "Using pre-built binary in container"
fi

# Set process limits for production
print_status "Setting process limits..."
ulimit -n 65536  # File descriptors
ulimit -u 32768  # Max user processes

# Create PID file
PID_FILE="/var/run/massive-graph.pid"
if [[ -w "$(dirname "$PID_FILE")" ]]; then
    echo $$ > "$PID_FILE"
    print_status "PID file created: $PID_FILE"
fi

# Setup signal handlers for graceful shutdown
cleanup() {
    print_status "Received shutdown signal, cleaning up..."
    if [[ -f "$PID_FILE" ]]; then
        rm -f "$PID_FILE"
    fi
    exit 0
}

trap cleanup SIGTERM SIGINT

# Run the application
print_status "Starting Massive Graph Database..."
print_status "Process ID: $$"

# Use the release binary directly for better performance
if [[ -f "./target/release/massive-graph" ]]; then
    exec ./target/release/massive-graph --config config.toml
else
    exec /usr/local/bin/massive-graph --config config.toml
fi 