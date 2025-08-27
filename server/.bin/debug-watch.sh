#!/bin/bash
# Debug-friendly development script for VS Code
# This script helps coordinate between cargo watch and VS Code debugging

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging configuration
LOG_DIR="./logs"
LOG_FILE="$LOG_DIR/massive-graph-server.log"

export RUST_LOG=debug
export MG_LOG_LEVEL=debug
export MG_DATA_DIR="./data"
export CARGO_TARGET_DIR="./server/target"

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

# Logging functions
print_log_info() {
    echo -e "${BLUE}[LOG]${NC} Logging to: $LOG_FILE"
    echo -e "${BLUE}[LOG]${NC} To tail logs: tail -f $LOG_FILE"
    echo -e "${BLUE}[LOG]${NC} To view recent: tail -n 100 $LOG_FILE"
}

setup_logging() {
    # Create logs directory if it doesn't exist
    mkdir -p "$LOG_DIR"
    
    # Initialize log file with header
    {
        echo "=========================================="
        echo "Massive Graph Server Development Log"
        echo "Started: $(date)"
        echo "=========================================="
        echo ""
    } > "$LOG_FILE"
    
    print_log_info
}

# Function to notify VS Code to restart debugging
notify_rebuild() {
    print_warning "Build completed. Restart VS Code debugger to attach to new process."
    # On macOS, we can use osascript to show a notification
    if command -v osascript &> /dev/null; then
        osascript -e 'display notification "Restart debugger to attach" with title "Cargo Build Complete"'
    fi
}



# Cleanup function
cleanup() {
    print_status "Cleaning up..."
    {
        echo ""
        echo "=========================================="
        echo "Server stopped: $(date)"
        echo "=========================================="
    } >> "$LOG_FILE"
}

# Set up trap for cleanup
trap cleanup EXIT INT TERM

# Setup logging
setup_logging

# Build once before starting watch
print_status "Initial build..."
if cargo build --manifest-path server/Cargo.toml --bin massive-graph-server 2>&1 | tee -a "$LOG_FILE"; then
    print_success "Initial build complete"
else
    print_error "Initial build failed"
    exit 1
fi

# Start cargo watch with notification on rebuild
print_status "Starting cargo watch with config.toml..."
print_warning "Use VS Code's 'Debug (attach to existing build)' configuration after rebuilds"
print_status "Using config file: config.toml"

# Log the start of cargo watch
{
    echo ""
    echo "=========================================="
    echo "Starting cargo watch: $(date)"
    echo "=========================================="
} >> "$LOG_FILE"

# Use exec to replace the shell with cargo watch, preserving signal handling
# Watch server source, core library, and config file
# Pipe all output to both terminal and log file
exec cargo watch \
    -w server/src \
    -w server/Cargo.toml \
    -w crates/massive-graph-core/src \
    -w config.toml \
    -x "run --manifest-path server/Cargo.toml --bin massive-graph-server -- --config config.toml" \
    --notify 2>&1 | tee -a "$LOG_FILE"