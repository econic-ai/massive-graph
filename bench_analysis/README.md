# Benchmark Analysis Tools

This directory contains tools for exporting, tracking, and analyzing Criterion benchmark results over time.

## Quick Start

### 1. Run Benchmarks
```bash
cd crates/massive-graph-core
cargo bench --bench compare_indexes_get_bench
```

### 2. Export Results to CSV
```bash
# From project root
python3 bench_analysis/export_bench_to_csv.py --tag "baseline"
```

### 3. Analyze in Jupyter
```bash
cd bench_analysis
jupyter notebook analyze_benchmarks.ipynb
```

---

## Files

- **`export_bench_to_csv.py`** - Script to export benchmark results to CSV
- **`analyze_benchmarks.ipynb`** - Jupyter notebook for analysis
- **`benchmark_results.csv`** - Output CSV file (created automatically)

---

## Export Script Usage

### Basic Usage
```bash
# Export latest results with a descriptive tag
python3 bench_analysis/export_bench_to_csv.py --tag "testing radix optimization"
```

### Advanced Options
```bash
# Export from 'base' folder (previous run)
python3 bench_analysis/export_bench_to_csv.py --tag "before changes" --source base

# Export only specific benchmarks
python3 bench_analysis/export_bench_to_csv.py --tag "get tests" --bench compare_get

# Custom output location
python3 bench_analysis/export_bench_to_csv.py --output my_results.csv
```

### Command-Line Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `--tag` | `""` | Descriptive tag for this benchmark run |
| `--source` | `"new"` | Source directory: `"new"` or `"base"` |
| `--bench` | `None` | Filter by benchmark name (partial match) |
| `--output` | `"bench_analysis/benchmark_results.csv"` | Output CSV file path |
| `--criterion-dir` | Auto-detect | Path to criterion directory |

---

## CSV Format

The exported CSV contains the following columns:

| Column | Description |
|--------|-------------|
| `run_id` | Unique ID based on timestamp (format: `YYYYMMDD_HHMMSS`) |
| `benchmark` | Benchmark group name (e.g., `compare_get_16b`) |
| `test_name` | Full test name (e.g., `hashmap_get/64`) |
| `operation` | Operation name (e.g., `hashmap_get`) |
| `size` | Input size parameter (e.g., `64`, `1024`) |
| `median_ns` | Median time in nanoseconds |
| `mean_ns` | Mean time in nanoseconds |
| `throughput_ops_sec` | Operations per second |
| `throughput_pretty` | Human-readable throughput (e.g., `10.5 Mops/s`) |
| `timestamp` | ISO format timestamp |
| `tag` | User-provided descriptive tag |

---

## Duplicate Detection

The script automatically detects and skips duplicates based on `run_id + test_name`. This means:

- ✅ Running the export script multiple times on the same results won't create duplicates
- ✅ You can safely re-run the script after partial benchmark runs
- ✅ Only new benchmark results (different timestamps) will be added

---

## Typical Workflow

### 1. Establish Baseline
```bash
# Run benchmarks
cargo bench --bench compare_indexes_get_bench

# Export with baseline tag
python3 bench_analysis/export_bench_to_csv.py --tag "baseline v1.0"
```

### 2. Make Code Changes
```rust
// Edit your code...
```

### 3. Run Benchmarks Again
```bash
cargo bench --bench compare_indexes_get_bench
```

### 4. Export New Results
```bash
python3 bench_analysis/export_bench_to_csv.py --tag "optimized radix index"
```

### 5. Analyze in Jupyter
```bash
cd bench_analysis
jupyter notebook analyze_benchmarks.ipynb
```

The notebook will:
- Automatically run the export script to get latest results
- Load all historical data from the CSV
- Provide example analysis and visualizations
- Allow you to compare performance across tags

---

## Tips

### Run Partial Benchmarks
If you're iterating on specific operations, you can run only those benchmarks:

```bash
# Run only specific benchmark
cargo bench --bench compare_indexes_get_bench

# Export only those results
python3 bench_analysis/export_bench_to_csv.py --tag "iteration 5" --bench compare_get
```

The script will skip tests that weren't re-run (same timestamp).

### Compare Before/After
```bash
# Export 'base' (previous run) before making changes
python3 bench_analysis/export_bench_to_csv.py --tag "before optimization" --source base

# Make changes and run benchmarks
cargo bench

# Export 'new' (current run) after changes
python3 bench_analysis/export_bench_to_csv.py --tag "after optimization" --source new
```

### Clean Start
If you want to start fresh with a new CSV:

```bash
rm bench_analysis/benchmark_results.csv
python3 bench_analysis/export_bench_to_csv.py --tag "fresh start"
```

---

## Analysis Examples

The Jupyter notebook includes examples for:

- Viewing all available operations and benchmarks
- Filtering by operation, size, or tag
- Comparing performance across different runs
- Plotting performance over time
- Analyzing performance vs input size
- Comparing tagged runs side-by-side

---

## Requirements

### Python Version
- Python 3.7 or higher

### Install Dependencies

```bash
# From bench_analysis directory
pip install -r requirements.txt

# Or manually
pip install pandas numpy matplotlib seaborn jupyter plotly
```

**Note**: The export script (`export_bench_to_csv.py`) has no dependencies and only requires Python 3.7+. Dependencies are only needed for the Jupyter notebook analysis.

---

## Troubleshooting

### "No benchmark results found"
- Make sure you've run `cargo bench` first
- Check that `target/criterion` directory exists
- Try specifying `--criterion-dir` explicitly

### "No new results found (skipped N duplicates)"
- This is normal if you haven't run benchmarks since last export
- Run `cargo bench` again to generate new results
- The script uses file modification timestamps to detect changes

### CSV shows unexpected results
- Check that you're exporting from the right source (`--source new` or `--source base`)
- Verify the benchmark ran successfully (check criterion output)
- Use `--bench` filter to narrow down to specific benchmarks

