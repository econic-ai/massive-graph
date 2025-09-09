#!/bin/bash
# Browser WASM watch script - rebuilds on file changes
# This script watches for changes and automatically rebuilds the WASM module

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging configuration
LOG_DIR="./logs"
LOG_FILE="$LOG_DIR/massive-graph-browser.log"

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

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Parse arguments
THREADED=false
RELEASE=false

show_usage() {
    echo "Usage: $0 [--threaded] [--release]"
    echo ""
    echo "Options:"
    echo "  --threaded    Use threaded WASM build (atomics, shared memory)"
    echo "  --release     Build in release mode"
    echo ""
    echo "Examples:"
    echo "  $0                    # Standard dev build"
    echo "  $0 --threaded         # Threaded dev build"
    echo "  $0 --release          # Standard release build"
    echo "  $0 --threaded --release # Threaded release build"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --threaded)
            THREADED=true
            shift
            ;;
        --release)
            RELEASE=true
            shift
            ;;
        --help|-h)
            show_usage
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Determine which build script to use
if [ "$THREADED" = true ]; then
    BUILD_SCRIPT="$SCRIPT_DIR/build-threaded.sh"
    BUILD_TYPE="threaded"
else
    BUILD_SCRIPT="$SCRIPT_DIR/build.sh"
    BUILD_TYPE="standard"
fi

# Determine build mode
BUILD_ARGS=""
if [ "$RELEASE" = true ]; then
    BUILD_ARGS="--release"
    BUILD_MODE="release"
else
    BUILD_MODE="dev"
fi

# Check if cargo-watch is installed
if ! command -v cargo-watch &> /dev/null; then
    print_error "cargo-watch is not installed!"
    print_status "Install it with: cargo install cargo-watch"
    exit 1
fi

# Check if build script exists
if [ ! -f "$BUILD_SCRIPT" ]; then
    print_error "Build script not found: $BUILD_SCRIPT"
    exit 1
fi

print_status "Starting WASM watch mode ($BUILD_TYPE $BUILD_MODE)..."
print_warning "Changes to browser or core library will trigger automatic rebuilds"
print_status "Built files will be in: browser/dist/"
print_status "Using build script: $(basename "$BUILD_SCRIPT")"

if [ "$THREADED" = true ]; then
    print_warning "Threaded builds require SharedArrayBuffer support in browser"
fi

# Create logs directory if it doesn't exist
mkdir -p "$LOG_DIR"

# Watch for changes and rebuild using the appropriate build script
exec cargo watch \
    -w "browser/src" \
    -w "browser/Cargo.toml" \
    -w "crates/massive-graph-core/src" \
    -w "crates/massive-graph-core/Cargo.toml" \
    -s "$BUILD_SCRIPT $BUILD_ARGS" \
    --notify 2>&1 | tee -a "$LOG_FILE"
