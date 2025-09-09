#!/bin/bash
# Browser WASM threaded build script - uses cargo directly for threading support

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
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

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BROWSER_DIR="$(dirname "$SCRIPT_DIR")"

# Determine build mode
BUILD_MODE=""
CARGO_MODE="debug"
if [ "${1:-}" = "--release" ]; then
    BUILD_MODE="--release"
    CARGO_MODE="release"
    print_status "Building threaded WASM module in RELEASE mode..."
else
    print_status "Building threaded WASM module in DEV mode..."
    print_status "Use '$0 --release' for production build"
fi

print_warning "This build uses advanced WASM features: atomics, shared memory, bulk memory"
print_warning "The resulting WASM will require SharedArrayBuffer support in the browser"

# Change to browser directory
cd "$BROWSER_DIR"

# Step 1: Build with cargo directly using build-std
print_status "Step 1: Building WASM with threading support using cargo..."

# Set threading-specific rustflags for this build only
export RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\" -C target-feature=+atomics,+bulk-memory,+mutable-globals -C link-arg=--shared-memory -C link-arg=--max-memory=67108864"

if ! CARGO_TARGET_DIR=./target cargo +nightly build \
    --target wasm32-unknown-unknown \
    $BUILD_MODE \
    -Z build-std=panic_abort,std \
    --lib; then
    print_error "Cargo build failed!"
    exit 1
fi

# Step 2: Generate bindings with wasm-bindgen
print_status "Step 2: Generating JavaScript bindings..."

# Find the generated wasm file
WASM_FILE="./target/wasm32-unknown-unknown/$CARGO_MODE/massive_graph_browser.wasm"
if [ ! -f "$WASM_FILE" ]; then
    print_error "WASM file not found: $WASM_FILE"
    exit 1
fi

# Create dist directory
mkdir -p dist

# Generate bindings
# Using web target for simpler integration
if ! wasm-bindgen \
    --target web \
    --out-dir dist \
    --out-name massive_graph_browser \
    "$WASM_FILE"; then
    print_error "wasm-bindgen failed!"
    exit 1
fi

# Note: For Option 2 (no-modules), replace above with:
# if ! wasm-bindgen \
#     --target no-modules \
#     --out-dir dist \
#     --out-name massive_graph_browser \
#     "$WASM_FILE"; then

# Step 3: Generate package.json
print_status "Step 3: Generating package.json..."
cat > dist/package.json << EOF
{
  "name": "@econic/massive-graph-browser",
  "version": "0.1.0",
  "description": "Massive Graph browser WASM package with threading support",
  "main": "massive_graph_browser.js",
  "types": "massive_graph_browser.d.ts",
  "files": [
    "massive_graph_browser.js",
    "massive_graph_browser.wasm",
    "massive_graph_browser.d.ts"
  ],
  "keywords": ["wasm", "threading", "shared-memory"],
  "license": "MIT"
}
EOF

# Step 4: Clean up
if [ -f "dist/.gitignore" ]; then
    rm dist/.gitignore
    print_status "Removed auto-generated .gitignore"
fi

print_success "Threaded WASM build complete!"
print_status "Output files in: $BROWSER_DIR/dist/"
print_warning "Remember: This WASM requires SharedArrayBuffer support"
print_warning "Serve with proper COOP/COEP headers for SharedArrayBuffer access"
