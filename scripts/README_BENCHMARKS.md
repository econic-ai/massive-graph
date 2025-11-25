# Benchmark Results Capture & Analysis

This document explains how to capture benchmark results and convert them to tables/graphs for analysis.

## Quick Start

### 1. Run Benchmarks
```bash
# From massive-graph root
cd crates/massive-graph-core
cargo bench

# Or run specific benchmark
cargo bench --bench compare_indexes_get_bench
```

### 2. Convert Results to Table/CSV

**Option A: Use the Python script (Recommended)**
```bash
# Generate Markdown table and CSV
python3 ../../scripts/parse_bench_results.py

# Generate with HTML interactive charts
python3 ../../scripts/parse_bench_results.py --html

# Filter specific benchmark
python3 ../../scripts/parse_bench_results.py --bench compare_get
```

**Option B: Use Criterion's JSON directly**

Criterion automatically saves results to `target/criterion/<benchmark>/<test>/base/estimates.json`

You can parse these JSON files directly for custom analysis.

## Output Files

After running the script, you'll find:

- `bench_results/BENCHMARK_RESULTS.md` - Markdown table for docs
- `bench_results/benchmark_results.csv` - CSV for Excel/analysis
- `bench_results/benchmark_results.html` - Interactive charts (if --html used)

## Criterion's Built-in Reports

Criterion already generates HTML reports automatically:

```bash
# View in browser
open target/criterion/report/index.html
```

These reports include:
- Detailed timing statistics
- Comparison with previous runs
- Performance regression detection
- Violin plots and histograms

## Adding CSV Export to New Benchmarks

If you want benchmarks to export CSV directly during the run, add this to your benchmark:

```rust
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};

fn my_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_test");
    
    // Enable CSV output (Criterion feature)
    group.sample_size(100);
    
    for &n in &[64, 1024, 10000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                // Your code here
            });
        });
    }
    
    group.finish();
}

criterion_group!(benches, my_benchmark);
criterion_main!(benches);
```

## Comparing Results Over Time

Criterion automatically saves historical data. To see how performance changes:

1. Run benchmarks: `cargo bench`
2. Make code changes
3. Run benchmarks again: `cargo bench`
4. Criterion will show % change in performance

To save a baseline for comparison:

```bash
# Save current results as baseline
cargo bench -- --save-baseline my-baseline

# Later, compare against baseline
cargo bench -- --baseline my-baseline
```

## Generating Reports from Historical Data

```bash
# Parse all historical runs
python3 scripts/parse_bench_results.py --criterion-dir target/criterion

# Generate comparison report
criterion-compare baseline new --export csv
```

## Tips

1. **Always run benchmarks multiple times** to ensure stable results
2. **Close other applications** when benchmarking to reduce noise
3. **Use --sample-size** to control measurement accuracy vs speed
4. **Git commit before major changes** so you can compare performance

## Example: Complete Workflow

```bash
# 1. Run benchmarks
cd crates/massive-graph-core
cargo bench --bench compare_indexes_get_bench

# 2. Generate reports
cd ../..
python3 scripts/parse_bench_results.py --html

# 3. View results
open bench_results/BENCHMARK_RESULTS.md
open bench_results/benchmark_results.html

# 4. Check Criterion's HTML report
open crates/massive-graph-core/target/criterion/report/index.html
```

## Installing plotly (for HTML charts)

```bash
pip3 install plotly
```

## Alternative: Criterion Table Output

You can also configure Criterion to print tables to console:

```rust
use criterion::{Criterion, BenchmarkGroup};
use criterion::measurement::WallTime;

fn my_benchmark(c: &mut Criterion) {
    // This will print a summary table
    c.bench_function("test", |b| b.iter(|| { /* code */ }));
}
```

The console output already includes timing tables that you can copy-paste.

