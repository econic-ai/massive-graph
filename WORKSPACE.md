# Massive Graph Workspace Structure

This project uses a Cargo workspace to manage multiple related packages.

## Workspace Members

1. **crates/massive-graph-core** - Shared core library (WASM-compatible)
   - Contains common types, traits, and utilities
   - Designed to work in both native and WASM environments
   - Minimal dependencies for maximum compatibility

2. **server** - Native server implementation
   - REST API and WebSocket server
   - Full tokio runtime with networking
   - Native-only features (file I/O, networking, etc.)

3. **browser** - WASM browser build
   - WebAssembly bindings for browser environments
   - Implements storage traits for browser use
   - Builds to browser/pkg/ for web integration

## Root Cargo.toml Purpose

The root `Cargo.toml` serves as:

1. **Workspace Definition** - Lists all member packages
2. **Dependency Management** - Centralizes version control for shared dependencies
3. **Build Profiles** - Defines optimization settings for all packages

### Shared Dependencies

Only dependencies used by multiple packages are defined at the workspace level:
- Core serialization (serde, serde_json)
- Common utilities (uuid, chrono, thiserror)
- WASM tooling (wasm-bindgen ecosystem)
- Minimal async runtime (tokio with no default features)

### Why Keep Server Dependencies?

Some server-only dependencies (anyhow, once_cell, hex, base64) are kept in workspace.dependencies because:
- They might be needed by future crates
- Centralizing versions prevents conflicts
- Easy to move if more packages need them

## Building

Each package can be built independently:
- Server: `cargo build --bin massive-graph-server`
- Browser: `./browser/.bin/build.sh`
- Core: Built as a dependency of others

The workspace ensures consistent dependency versions across all packages.
