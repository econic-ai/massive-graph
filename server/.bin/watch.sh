#!/bin/bash
# Active development script with file watching for Massive Graph Database

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

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    print_error "Must be run from the massive-graph directory"
    exit 1
fi

# Set development environment variables
export RUST_LOG=debug
export MG_LOG_LEVEL=debug
export MG_DATA_DIR="./data"
export MG_HTTP_ADDR="${MG_HTTP_ADDR:-0.0.0.0:8080}"
export MG_WS_ADDR="127.0.0.1:8080"
export MG_QUIC_ADDR="127.0.0.1:8080"
export MG_WORKER_THREADS=4

print_status "Starting Massive Graph Database in development mode..."
print_status "Configuration:"
echo "  - HTTP Server: $MG_HTTP_ADDR"
echo "  - WebSocket Server: $MG_WS_ADDR"
echo "  - QUIC Server: $MG_QUIC_ADDR"
echo "  - Data Directory: $MG_DATA_DIR"
echo "  - Worker Threads: $MG_WORKER_THREADS"
echo "  - Log Level: $MG_LOG_LEVEL"

# Create data directory if it doesn't exist
mkdir -p "$MG_DATA_DIR"

# Check if ports are available
check_port() {
    local port=$1
    local name=$2
    if lsof -Pi :$port -sTCP:LISTEN -t >/dev/null 2>&1; then
        print_error "$name port $port is already in use"
        return 1
    fi
}

# Extract port numbers from addresses
HTTP_PORT=$(echo $MG_HTTP_ADDR | cut -d':' -f2)
WS_PORT=$(echo $MG_WS_ADDR | cut -d':' -f2)
QUIC_PORT=$(echo $MG_QUIC_ADDR | cut -d':' -f2)

print_status "Checking port availability..."
check_port $HTTP_PORT "HTTP" || exit 1
check_port $WS_PORT "WebSocket" || exit 1
check_port $QUIC_PORT "QUIC" || exit 1

print_success "All ports are available"

# Build in debug mode for faster compilation
print_status "Building in debug mode..."
if ! cargo build; then
    print_error "Build failed"
    exit 1
fi

print_success "Build completed"

# Run the application with file watching
print_status "Starting Massive Graph Database with file watching..."
print_warning "Press Ctrl+C to stop the server"
print_status "Files will be watched for changes and the server will restart automatically"

# Trap Ctrl+C to cleanup
trap 'print_status "Shutting down..."; exit 0' INT

# Use cargo-watch to monitor file changes and restart
cargo watch -x "run -- --config config.toml" -w config.toml 