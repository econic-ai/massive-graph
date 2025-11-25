#!/usr/bin/env python3
"""
Parse Criterion benchmark results from target/criterion and generate:
- Markdown table
- CSV export
- Optional HTML charts (if plotly installed)

Usage:
    python scripts/parse_bench_results.py                    # Parse all benchmarks
    python scripts/parse_bench_results.py --bench compare_get # Parse specific benchmark
    python scripts/parse_bench_results.py --html              # Generate HTML charts
"""

import json
import csv
import argparse
from pathlib import Path
from collections import defaultdict
from typing import Dict, List, Tuple


def parse_criterion_results(criterion_dir: Path, bench_filter: str = None) -> List[Dict]:
    """Parse Criterion's JSON output from target/criterion directory"""
    results = []
    
    if not criterion_dir.exists():
        print(f"Warning: {criterion_dir} does not exist. Run 'cargo bench' first.")
        return results
    
    # Walk through criterion directory structure
    for benchmark_dir in criterion_dir.iterdir():
        if not benchmark_dir.is_dir():
            continue
            
        # Skip if filtering and doesn't match
        if bench_filter and bench_filter not in str(benchmark_dir.name):
            continue
            
        # Look for subdirectories (test groups)
        for test_dir in benchmark_dir.iterdir():
            if not test_dir.is_dir():
                continue
                
            estimates_file = test_dir / "base" / "estimates.json"
            source = "base"
            if not estimates_file.exists():
                # Try new directory structure
                estimates_file = test_dir / "new" / "estimates.json"
                source = "new"
                
            if estimates_file.exists():
                try:
                    with open(estimates_file) as f:
                        data = json.load(f)
                    
                    # Get file modification time
                    import os
                    from datetime import datetime
                    mtime = os.path.getmtime(estimates_file)
                    timestamp = datetime.fromtimestamp(mtime).isoformat()
                        
                    # Extract benchmark name, test name from path
                    bench_name = benchmark_dir.name
                    test_name = test_dir.name
                    
                    # Parse test name to extract parameters
                    parts = test_name.split('/')
                    operation = parts[0] if parts else test_name
                    size = parts[1] if len(parts) > 1 else "N/A"
                    
                    # Get median estimate (point estimate)
                    median_ns = data.get("median", {}).get("point_estimate", 0)
                    mean_ns = data.get("mean", {}).get("point_estimate", 0)
                    
                    results.append({
                        "benchmark": bench_name,
                        "test": test_name,
                        "operation": operation,
                        "size": size,
                        "median_ns": median_ns,
                        "mean_ns": mean_ns,
                        "median_us": median_ns / 1000,
                        "median_ms": median_ns / 1_000_000,
                        "timestamp": timestamp,
                        "source": source,
                    })
                except Exception as e:
                    print(f"Warning: Failed to parse {estimates_file}: {e}")
    
    return results


def format_time(ns: float) -> Tuple[float, str]:
    """Format time in appropriate unit"""
    if ns < 1000:
        return ns, "ns"
    elif ns < 1_000_000:
        return ns / 1000, "µs"
    elif ns < 1_000_000_000:
        return ns / 1_000_000, "ms"
    else:
        return ns / 1_000_000_000, "s"


def format_throughput(ns: float, n: int = 1) -> str:
    """Calculate and format operations per second"""
    if ns == 0:
        return "N/A"
    
    ops_per_sec = (n * 1_000_000_000) / ns
    
    if ops_per_sec >= 1_000_000_000:
        return f"{ops_per_sec / 1_000_000_000:.2f} Gops/s"
    elif ops_per_sec >= 1_000_000:
        return f"{ops_per_sec / 1_000_000:.2f} Mops/s"
    elif ops_per_sec >= 1_000:
        return f"{ops_per_sec / 1_000:.2f} Kops/s"
    else:
        return f"{ops_per_sec:.2f} ops/s"


def generate_markdown_table(results: List[Dict]) -> str:
    """Generate markdown table from benchmark results"""
    if not results:
        return "No benchmark results found."
    
    # Group by benchmark
    by_benchmark = defaultdict(list)
    for r in results:
        by_benchmark[r["benchmark"]].append(r)
    
    markdown = ["# Benchmark Results\n"]
    
    for bench_name, bench_results in sorted(by_benchmark.items()):
        markdown.append(f"\n## {bench_name}\n")
        markdown.append("| Operation | Size | Median Time | Throughput |")
        markdown.append("|-----------|------|-------------|------------|")
        
        for r in sorted(bench_results, key=lambda x: (x["operation"], x["size"])):
            time_val, time_unit = format_time(r["median_ns"])
            
            # Try to extract size for throughput calculation
            try:
                size_n = int(r["size"]) if r["size"].isdigit() else 1
            except:
                size_n = 1
                
            throughput = format_throughput(r["median_ns"], size_n)
            
            markdown.append(f"| {r['operation']} | {r['size']} | {time_val:.2f} {time_unit} | {throughput} |")
    
    return "\n".join(markdown)


def export_csv(results: List[Dict], output_path: Path, append: bool = False):
    """Export results to CSV"""
    if not results:
        print("No results to export")
        return
    
    # Add timestamp to each result if appending
    if append:
        from datetime import datetime
        timestamp = datetime.now().isoformat()
        for r in results:
            r["timestamp"] = timestamp
        
    fieldnames = [
        "timestamp", "benchmark", "test", "operation", "size", 
        "median_ns", "mean_ns", "median_us", "median_ms"
    ] if append else [
        "benchmark", "test", "operation", "size", 
        "median_ns", "mean_ns", "median_us", "median_ms"
    ]
    
    mode = 'a' if (append and output_path.exists()) else 'w'
    write_header = not (append and output_path.exists())
    
    with open(output_path, mode, newline='') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        if write_header:
            writer.writeheader()
        writer.writerows(results)
    
    action = "Appended to" if append else "Exported"
    print(f"✓ {action} CSV: {output_path}")


def generate_html_chart(results: List[Dict], output_path: Path):
    """Generate interactive HTML chart using plotly"""
    try:
        import plotly.graph_objects as go
        from plotly.subplots import make_subplots
    except ImportError:
        print("Warning: plotly not installed. Install with: pip install plotly")
        return
    
    if not results:
        print("No results to chart")
        return
    
    # Group by operation and benchmark
    by_operation = defaultdict(lambda: defaultdict(list))
    for r in results:
        by_operation[r["operation"]][r["benchmark"]].append(r)
    
    # Create subplot for each operation
    fig = make_subplots(
        rows=len(by_operation),
        cols=1,
        subplot_titles=[op for op in sorted(by_operation.keys())]
    )
    
    for idx, (operation, benchmarks) in enumerate(sorted(by_operation.items()), start=1):
        for bench_name, bench_results in sorted(benchmarks.items()):
            # Sort by size
            sorted_results = sorted(bench_results, key=lambda x: int(x["size"]) if x["size"].isdigit() else 0)
            
            sizes = [r["size"] for r in sorted_results]
            times_us = [r["median_us"] for r in sorted_results]
            
            fig.add_trace(
                go.Scatter(
                    x=sizes,
                    y=times_us,
                    mode='lines+markers',
                    name=f"{bench_name}",
                    legendgroup=bench_name,
                    showlegend=(idx == 1)  # Only show legend for first subplot
                ),
                row=idx,
                col=1
            )
        
        fig.update_xaxis(title_text="Input Size", row=idx, col=1)
        fig.update_yaxis(title_text="Time (µs)", type="log", row=idx, col=1)
    
    fig.update_layout(
        height=300 * len(by_operation),
        title_text="Benchmark Results",
        showlegend=True
    )
    
    fig.write_html(str(output_path))
    print(f"✓ Generated HTML chart: {output_path}")


def main():
    parser = argparse.ArgumentParser(description="Parse Criterion benchmark results")
    parser.add_argument("--bench", help="Filter by benchmark name")
    parser.add_argument("--html", action="store_true", help="Generate HTML charts")
    parser.add_argument("--output-dir", default="bench_results", help="Output directory")
    parser.add_argument("--criterion-dir", help="Path to criterion directory (default: auto-detect)")
    parser.add_argument("--append-history", action="store_true", help="Append results to historical CSV with timestamp")
    
    args = parser.parse_args()
    
    # Find criterion directory
    if args.criterion_dir:
        criterion_dir = Path(args.criterion_dir)
    else:
        # Try to find it from common locations
        repo_root = Path(__file__).parent.parent
        criterion_dir = repo_root / "target" / "criterion"
        
        if not criterion_dir.exists():
            criterion_dir = repo_root / "crates" / "massive-graph-core" / "target" / "criterion"
    
    # Parse results
    print(f"Parsing benchmark results from {criterion_dir}...")
    results = parse_criterion_results(criterion_dir, args.bench)
    
    if not results:
        print("No benchmark results found. Run 'cargo bench' first.")
        return
    
    print(f"Found {len(results)} benchmark results")
    
    # Create output directory
    output_dir = Path(args.output_dir)
    output_dir.mkdir(exist_ok=True)
    
    # Generate outputs
    markdown = generate_markdown_table(results)
    md_path = output_dir / "BENCHMARK_RESULTS.md"
    with open(md_path, 'w') as f:
        f.write(markdown)
    print(f"✓ Generated Markdown table: {md_path}")
    
    # Export CSV
    if args.append_history:
        csv_path = output_dir / "benchmark_history.csv"
        export_csv(results, csv_path, append=True)
    else:
        csv_path = output_dir / "benchmark_results.csv"
        export_csv(results, csv_path, append=False)
    
    # Generate HTML chart if requested
    if args.html:
        html_path = output_dir / "benchmark_results.html"
        generate_html_chart(results, html_path)
    
    # Print summary to console
    print("\n" + "="*80)
    print(markdown)


if __name__ == "__main__":
    main()

