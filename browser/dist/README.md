# Massive Graph Browser WASM

This package contains the WebAssembly build of Massive Graph for browser environments.

## Development

### Prerequisites

- Rust with wasm32-unknown-unknown target
- wasm-pack (install with `curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh`)
- cargo-watch (install with `cargo install cargo-watch`)

### Building

For development with automatic rebuilds on file changes:
```bash
# From project root
./browser/.bin/watch.sh
```

For a one-time development build:
```bash
# From project root
./browser/.bin/build.sh
```

For production build:
```bash
# From project root
./browser/.bin/build.sh --release
```

### Testing

After building, you can serve the files locally for testing:
```bash
cd browser/pkg
python3 -m http.server 8080
```

Then open http://localhost:8080 in your browser. The test page (index.html) will:
- Load the WASM module
- Initialize it (showing the hello world message from core)
- Display the version information
- Create a test storage instance

### Integration

The built files in `pkg/` can be imported in any web application:

```javascript
import init, { WasmStorageWrapper, version } from './pkg/massive_graph_wasm.js';

async function setupWasm() {
    await init(); // This will log "WASM initialized: Hello from Massive Graph Core!"
    
    console.log('Version:', version());
    
    const storage = new WasmStorageWrapper();
    // Use storage methods...
}
```

### Output Files

The build process generates:
- `pkg/massive_graph_wasm.js` - JavaScript bindings
- `pkg/massive_graph_wasm_bg.wasm` - WASM binary
- `pkg/massive_graph_wasm.d.ts` - TypeScript definitions
- `pkg/package.json` - NPM package metadata
