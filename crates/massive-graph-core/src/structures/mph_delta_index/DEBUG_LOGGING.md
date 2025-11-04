# Debug Logging System

This module provides flexible, zero-overhead debug logging that can be controlled via environment variables during development.

## Quick Start

### Enable Debug Logging

No need to modify `Cargo.toml`! Just set environment variables before running:

```bash
# Enable ALL debug logging
MG_DEBUG=1 cargo run

# Enable logging for specific modules
MG_DEBUG=mph_delta_index cargo run

# Enable logging for index-related operations (convenience flag)
MG_DEBUG_INDEX=1 cargo run

# Enable logging for specific files
MG_DEBUG=optimised_index.rs cargo run
```

### Compile-Time Control

You can still use the `debug-logging` feature flag for global compile-time control:

```bash
# Compile with debug logging enabled
cargo run --features debug-logging

# Compile with debug logging disabled (zero overhead)
cargo run
```

## Usage in Code

### Basic Debug Logging

```rust
use crate::debug_log;

// Simple debug log - uses module path automatically
debug_log!("upsert: key={:?}, idx={}", key, idx);

// Explicit module path (more control)
debug_log!(module = "mph_delta_index", "key={:?}", key);
```

### Labeled Debug Logging

```rust
use crate::debug_log_labeled;

// Add a label prefix
debug_log_labeled!("UPSERT", "key={:?}, idx={}", key, idx);
// Output: [UPSERT] key=..., idx=...

// With explicit module path
debug_log_labeled!(module = "mph_delta_index", "UPSERT", "key={:?}", key);
```

### Conditional Evaluation

```rust
use crate::debug_eval;

// Only compute expensive operation when debugging is enabled
let expensive_value = debug_eval!(module = "mph_delta_index", compute_expensive_thing());
debug_log!("value={:?}", expensive_value);

// Without explicit module
let expensive_value = debug_eval!(compute_expensive_thing());
```

### Debug Blocks

```rust
use crate::debug_block;

// Entire block only executes when debugging is enabled
debug_block!(module = "mph_delta_index") {
    let x = expensive_computation();
    let y = another_expensive_thing();
    eprintln!("x={}, y={}", x, y);
}

// Without explicit module
debug_block! {
    let x = expensive_computation();
    debug_log!("x={}", x);
}
```

## Environment Variables

### MG_DEBUG

Controls general debug logging:

- `MG_DEBUG=1` or `MG_DEBUG=all` - Enable all debug logging
- `MG_DEBUG=mph_delta_index` - Enable logging for modules containing "mph_delta_index"
- `MG_DEBUG=index` - Enable logging for any module containing "index"
- `MG_DEBUG=radix_index.rs` - Enable logging for specific file

**Pattern Matching**: Uses substring matching, so `MG_DEBUG=index` will match any module path containing "index".

### MG_DEBUG_INDEX

Convenience flag for index-related debugging:

- `MG_DEBUG_INDEX=1` - Equivalent to enabling `mph_delta_index` and `optimised_index` patterns

## Examples

### Debug a Specific Function

```bash
# Enable logging only for the upsert function
MG_DEBUG=mph_delta_index::upsert cargo run
```

### Debug Multiple Modules

```bash
# Enable logging for any module containing "index" or "radix"
MG_DEBUG=index MG_DEBUG=radix cargo run
```

Note: The last MG_DEBUG value takes precedence. For multiple patterns, use the contains-based matching.

### Production Build

```bash
# No environment variables = zero overhead
cargo run --release
```

All debug logging compiles to nothing when `debug-logging` feature is disabled.

## Extending to Other Modules

To use this system in other modules:

1. Add the debug-logging feature flag to your crate's `Cargo.toml` (already present)
2. Import the macros at the top of your module:

```rust
use crate::debug_log;
use crate::debug_log_labeled;
use crate::debug_eval;
use crate::debug_block;
```

3. Use the macros in your code
4. Control via environment variables as shown above

## Performance

- **When disabled**: Zero overhead - code compiles to nothing
- **When enabled**: Minimal runtime cost - just a string comparison per log call
- **First access**: Configuration is lazy-loaded on first use via `OnceLock`

## Tips

1. Use `module = "..."` parameter for precise control over which logs appear
2. Default behavior uses `module_path!()` and `file!()` for automatic context
3. For production builds, ensure `debug-logging` feature is disabled
4. Combine with cargo watch for rapid iteration: `MG_DEBUG=index cargo watch -x run`

