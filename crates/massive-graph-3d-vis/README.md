# High-Performance 3D Graph Visualisation Framework

A next-generation browser-based 3D graph visualisation framework capable of efficiently rendering and manipulating graphs containing millions of nodes and edges. Built with Rust and WebAssembly for near-native performance, utilising WebGPU for hardware-accelerated rendering.

![Build Status](https://img.shields.io/badge/build-no%20CI%20configured-lightgrey.svg)
![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)

## Features

- **Zero-copy architecture** from server to GPU rendering pipeline
- **High performance**: 60+ FPS with 1-10M nodes
- **Real-time collaboration** with sub-100ms update latency
- **WebGPU rendering** for hardware acceleration
- **WebAssembly core** compiled from Rust for near-native performance
- **Minimal memory footprint** through efficient data structures

## Quick Start

### Prerequisites

- Rust 1.70+
- Node.js 18+
- `wasm-pack` for WebAssembly compilation

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Or via cargo
cargo install wasm-pack
```

### Building the WebAssembly Module

```bash
# Clone the repository
git clone https://github.com/econic-ai/3d-vis.git
cd 3d-vis

# Build the WebAssembly package
wasm-pack build --target web --out-dir pkg

# For bundler integration (webpack, vite, etc.)
wasm-pack build --target bundler --out-dir pkg-bundler
```

## Usage

### Plain HTML/JavaScript

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>3D Graph Visualisation</title>
    <style>
        canvas { width: 100%; height: 100vh; }
    </style>
</head>
<body>
    <canvas id="graph-canvas"></canvas>
    
    <script type="module">
        import init, { GraphRenderer } from './pkg/3d_vis.js';
        
        async function run() {
            // Initialize the wasm module
            await init();
            
            // Create renderer
            const canvas = document.getElementById('graph-canvas');
            const renderer = await GraphRenderer.new(canvas);
            
            // Add some nodes
            renderer.add_node(0, 0.0, 0.0, 0.0, 0xff0000);
            renderer.add_node(1, 1.0, 1.0, 1.0, 0x00ff00);
            renderer.add_edge(0, 1, 1.0);
            
            // Start render loop
            renderer.start_render_loop();
        }
        
        run();
    </script>
</body>
</html>
```

### SvelteKit Integration

```bash
# Install the package
npm install ./pkg-bundler
```

```svelte
<!-- GraphVisualisation.svelte -->
<script>
    import { onMount } from 'svelte';
    import init, { GraphRenderer } from '3d-vis';
    
    let canvas;
    let renderer;
    
    onMount(async () => {
        // Initialize WASM
        await init();
        
        // Create renderer
        renderer = await GraphRenderer.new(canvas);
        
        // Load your graph data
        loadGraphData();
        
        // Start rendering
        renderer.start_render_loop();
        
        return () => {
            if (renderer) {
                renderer.destroy();
            }
        };
    });
    
    function loadGraphData() {
        // Example: Add 1000 random nodes
        for (let i = 0; i < 1000; i++) {
            const x = (Math.random() - 0.5) * 10;
            const y = (Math.random() - 0.5) * 10;
            const z = (Math.random() - 0.5) * 10;
            const colour = Math.floor(Math.random() * 0xffffff);
            
            renderer.add_node(i, x, y, z, colour);
        }
        
        // Add random edges
        for (let i = 0; i < 500; i++) {
            const source = Math.floor(Math.random() * 1000);
            const target = Math.floor(Math.random() * 1000);
            if (source !== target) {
                renderer.add_edge(source, target, 1.0);
            }
        }
    }
    
    function handleNodeClick(nodeId) {
        console.log('Node clicked:', nodeId);
    }
</script>

<canvas 
    bind:this={canvas}
    on:click={handleNodeClick}
    style="width: 100%; height: 500px;"
></canvas>
```

```javascript
// vite.config.js - Add WebAssembly support
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
    plugins: [sveltekit()],
    server: {
        fs: {
            allow: ['..']
        },
        headers: {
            'Cross-Origin-Embedder-Policy': 'require-corp',
            'Cross-Origin-Opener-Policy': 'same-origin'
        }
    },
    optimizeDeps: {
        exclude: ['3d-vis']
    }
});
```

## API Reference

### GraphRenderer

```typescript
class GraphRenderer {
    // Create new renderer instance
    static async new(canvas: HTMLCanvasElement): Promise<GraphRenderer>;
    
    // Node management
    add_node(id: number, x: number, y: number, z: number, colour: number): void;
    update_node_position(id: number, x: number, y: number, z: number): void;
    remove_node(id: number): void;
    
    // Edge management
    add_edge(source: number, target: number, weight: number): void;
    remove_edge(source: number, target: number): void;
    
    // Rendering
    start_render_loop(): void;
    stop_render_loop(): void;
    render_frame(): void;
    
    // Camera controls
    set_camera_position(x: number, y: number, z: number): void;
    set_camera_target(x: number, y: number, z: number): void;
    
    // Cleanup
    destroy(): void;
}
```

## Performance Targets

| Nodes | Edges | Target FPS | Memory Usage |
|-------|-------|------------|--------------|
| 100K  | 500K  | 60+ FPS    | < 100MB      |
| 1M    | 5M    | 60+ FPS    | < 500MB      |
| 10M   | 50M   | 30+ FPS    | < 2GB        |

## Browser Requirements

- **WebGPU support** (Chrome 113+, Firefox with flag, Safari TP)
- **WebAssembly** (all modern browsers)
- **SharedArrayBuffer** for optimal performance (secure context required)

### WebGPU Availability

Check WebGPU support:
```javascript
if ('gpu' in navigator) {
    console.log('WebGPU is supported');
} else {
    console.log('WebGPU not available - consider WebGL fallback');
}
```

## Development

### Building for Development

```bash
# Development build with debug symbols
wasm-pack build --dev --target web

# Watch mode (requires cargo-watch)
cargo install cargo-watch
cargo watch -x 'build'
```

### Running Examples

```bash
# Serve examples locally
python -m http.server 8000
# or
npx serve .

# Open http://localhost:8000/examples/
```

### Testing

```bash
# Run Rust tests
cargo test

# Run browser tests (requires headless Chrome)
wasm-pack test --chrome --headless
```

## Research Background

This project implements the research described in our paper on high-performance 3D graph visualisation. The framework addresses fundamental limitations in current JavaScript-based visualisation tools through:

- Zero-copy memory architecture from network to GPU
- Rust/WebAssembly for computational performance
- WebGPU for hardware-accelerated rendering
- Real-time collaborative editing capabilities

For detailed technical background, see our [research paper](docs/research-paper.md).

## Use Cases

- **Digital Twins**: Real-time sensor network visualisation
- **Collaborative Analytics**: Multi-user data exploration
- **Intelligence Networks**: Information flow visualisation
- **Financial Networks**: Market and trading visualisations
- **Scientific Computing**: Molecular and network analysis

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md).

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for new functionality
5. Run the test suite (`cargo test && wasm-pack test --chrome --headless`)
6. Commit your changes (`git commit -m 'Add amazing feature'`)
7. Push to the branch (`git push origin feature/amazing-feature`)
8. Open a Pull Request

## License

Apache 2.0 - see [LICENSE](LICENSE) for details.

## Support

- **Issues**: [GitHub Issues](https://github.com/econic-ai/3d-vis/issues)
- **Discussions**: [GitHub Discussions](https://github.com/econic-ai/3d-vis/discussions)
- **Documentation**: [docs.econic.ai/3d-vis](https://docs.econic.ai/3d-vis)

---

*Part of the [Econic AI](https://econic.ai) technology stack*