#!/bin/bash
# Browser WASM build script - one-off build

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    print_error "wasm-pack is not installed!"
    print_status "Install it with: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
fi

# Determine build mode
BUILD_MODE="--dev"
if [ "${1:-}" = "--release" ]; then
    BUILD_MODE="--release"
    print_status "Building WASM module in RELEASE mode..."
else
    print_status "Building WASM module in DEV mode..."
    print_status "Use '$0 --release' for production build"
fi

# Build from current directory (already in browser)
# Set target directory to be within browser folder
if CARGO_TARGET_DIR=./target wasm-pack build --target web --out-dir pkg --scope econic $BUILD_MODE; then
    print_success "WASM build complete!"
    print_status "Output files in: browser/pkg/"
else
    print_error "WASM build failed!"
    exit 1
fi
