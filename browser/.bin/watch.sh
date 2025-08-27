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

# Check if wasm-pack is installed
if ! command -v wasm-pack &> /dev/null; then
    print_error "wasm-pack is not installed!"
    print_status "Install it with: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh"
    exit 1
fi

# Check if cargo-watch is installed
if ! command -v cargo-watch &> /dev/null; then
    print_error "cargo-watch is not installed!"
    print_status "Install it with: cargo install cargo-watch"
    exit 1
fi

print_status "Starting WASM watch mode..."
print_warning "Changes to browser or core library will trigger automatic rebuilds"
print_status "Built files will be in: browser/dist/"

# Watch for changes and rebuild
# Note: Running from project root, watching browser and core library
# Set target directory to be within browser folder
exec cargo watch \
    -w "browser/src" \
    -w "browser/Cargo.toml" \
    -w "crates/massive-graph-core/src" \
    -w "crates/massive-graph-core/Cargo.toml" \
    -s "cd browser && CARGO_TARGET_DIR=./target wasm-pack build --target web --out-dir dist --scope econic --dev && rm -f dist/.gitignore" \
    --notify
