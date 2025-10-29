#!/usr/bin/env python3
"""
Export Criterion benchmark results to CSV for analysis.

This script reads benchmark results from target/criterion and exports them to a CSV file,
automatically detecting and skipping duplicates based on run_id (timestamp).

Usage:
    python export_bench_to_csv.py                              # Export from 'new' folder
    python export_bench_to_csv.py --tag "optimized radix"      # Add descriptive tag
    python export_bench_to_csv.py --source base                # Export from 'base' folder
    python export_bench_to_csv.py --bench compare_get          # Filter specific benchmark
"""

import json
import csv
import argparse
import os
from pathlib import Path
from datetime import datetime
from typing import Dict, List, Set


def get_run_id_from_file(file_path: Path) -> str:
    """Generate run_id from file modification timestamp"""
    mtime = os.path.getmtime(file_path)
    return datetime.fromtimestamp(mtime).strftime("%Y%m%d_%H%M%S")


def format_throughput(ns: float, n: int = 1) -> tuple[float, str]:
    """Calculate throughput and return (ops_per_sec, pretty_string)"""
    if ns == 0:
        return 0.0, "N/A"
    
    ops_per_sec = (n * 1_000_000_000) / ns
    
    if ops_per_sec >= 1_000_000_000:
        pretty = f"{ops_per_sec / 1_000_000_000:.2f} Gops/s"
    elif ops_per_sec >= 1_000_000:
        pretty = f"{ops_per_sec / 1_000_000:.2f} Mops/s"
    elif ops_per_sec >= 1_000:
        pretty = f"{ops_per_sec / 1_000:.2f} Kops/s"
    else:
        pretty = f"{ops_per_sec:.2f} ops/s"
    
    return ops_per_sec, pretty


def load_existing_run_ids(csv_path: Path) -> Set[str]:
    """Load existing run_id + test_name combinations to detect duplicates"""
    existing = set()
    
    if not csv_path.exists():
        return existing
    
    try:
        with open(csv_path, 'r', newline='') as f:
            reader = csv.DictReader(f)
            for row in reader:
                # Create unique key from run_id and test_name
                key = f"{row.get('run_id', '')}:{row.get('test_name', '')}"
                existing.add(key)
    except Exception as e:
        print(f"Warning: Could not read existing CSV: {e}")
    
    return existing


def parse_benchmark_results(
    criterion_dir: Path,
    source: str = "new",
    bench_filter: str = None,
    tag: str = ""
) -> List[Dict]:
    """Parse Criterion benchmark results from specified source directory"""
    results = []
    
    if not criterion_dir.exists():
        print(f"Error: {criterion_dir} does not exist. Run 'cargo bench' first.")
        return results
    
    # Walk through criterion directory
    for benchmark_dir in criterion_dir.iterdir():
        if not benchmark_dir.is_dir():
            continue
        
        # Apply filter if specified
        if bench_filter and bench_filter not in str(benchmark_dir.name):
            continue
        
        bench_name = benchmark_dir.name
        
        # Look for test subdirectories
        # Criterion can have either:
        # - benchmark_dir/test_name/source/estimates.json
        # - benchmark_dir/test_name/parameter/source/estimates.json
        
        for test_dir in benchmark_dir.iterdir():
            if not test_dir.is_dir():
                continue
            
            # First try direct path: test_dir/source/estimates.json
            estimates_file = test_dir / source / "estimates.json"
            
            if estimates_file.exists():
                # Process this result
                results.extend(process_estimates_file(
                    estimates_file, bench_name, test_dir.name, source, tag
                ))
            else:
                # Try one level deeper (for parameterized tests)
                for param_dir in test_dir.iterdir():
                    if not param_dir.is_dir():
                        continue
                    
                    estimates_file = param_dir / source / "estimates.json"
                    if estimates_file.exists():
                        # Combine test_name with parameter
                        full_test_name = f"{test_dir.name}/{param_dir.name}"
                        results.extend(process_estimates_file(
                            estimates_file, bench_name, full_test_name, source, tag
                        ))
    
    return results


def process_estimates_file(
    estimates_file: Path,
    bench_name: str,
    test_name: str,
    source: str,
    tag: str
) -> List[Dict]:
    """Process a single estimates.json file and return results"""
    results = []
    
    if not estimates_file.exists():
        return results
    
    try:
        # Load timing data
        with open(estimates_file) as f:
            data = json.load(f)
        
        # Get run_id from file timestamp
        run_id = get_run_id_from_file(estimates_file)
        
        # Get ISO timestamp for readability
        mtime = os.path.getmtime(estimates_file)
        timestamp = datetime.fromtimestamp(mtime).isoformat()
        
        # Parse test name to extract operation and size
        parts = test_name.split('/')
        operation = parts[0] if parts else test_name
        size = parts[1] if len(parts) > 1 else "N/A"
        
        # Extract timing metrics
        median_ns = data.get("median", {}).get("point_estimate", 0)
        mean_ns = data.get("mean", {}).get("point_estimate", 0)
        
        # Calculate throughput (use size if numeric, else 1)
        try:
            size_n = int(size) if size.isdigit() else 1
        except:
            size_n = 1
        
        throughput_ops_sec, throughput_pretty = format_throughput(median_ns, size_n)
        
        # Build result record
        results.append({
            "run_id": run_id,
            "benchmark": bench_name,
            "test_name": test_name,
            "operation": operation,
            "size": size,
            "median_ns": round(median_ns, 2),
            "mean_ns": round(mean_ns, 2),
            "throughput_ops_sec": round(throughput_ops_sec, 2),
            "throughput_pretty": throughput_pretty,
            "timestamp": timestamp,
            "tag": tag,
        })
        
    except Exception as e:
        print(f"Warning: Failed to parse {estimates_file}: {e}")
    
    return results


def export_to_csv(results: List[Dict], csv_path: Path, existing_ids: Set[str]):
    """Export results to CSV, skipping duplicates"""
    if not results:
        print("No new results to export")
        return 0
    
    # Filter out duplicates
    new_results = []
    skipped = 0
    
    for result in results:
        key = f"{result['run_id']}:{result['test_name']}"
        if key in existing_ids:
            skipped += 1
            continue
        new_results.append(result)
        existing_ids.add(key)  # Track for this session
    
    if not new_results:
        print(f"No new results found (skipped {skipped} duplicates)")
        return 0
    
    # Define column order
    fieldnames = [
        "run_id", "benchmark", "test_name", "operation", "size",
        "median_ns", "mean_ns", "throughput_ops_sec", "throughput_pretty",
        "timestamp", "tag"
    ]
    
    # Check if we need to write header
    file_exists = csv_path.exists() and csv_path.stat().st_size > 0
    
    # Append to CSV
    with open(csv_path, 'a', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        
        if not file_exists:
            writer.writeheader()
        
        writer.writerows(new_results)
    
    print(f"✓ Exported {len(new_results)} new results to {csv_path}")
    if skipped > 0:
        print(f"  Skipped {skipped} duplicate results")
    
    return len(new_results)


def main():
    parser = argparse.ArgumentParser(
        description="Export Criterion benchmark results to CSV",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s --tag "baseline performance"
  %(prog)s --source base --tag "before optimization"
  %(prog)s --bench compare_get --tag "radix index v2"
        """
    )
    
    parser.add_argument(
        "--tag",
        default="",
        help="Descriptive tag for this benchmark run (e.g., 'testing change B')"
    )
    parser.add_argument(
        "--source",
        choices=["new", "base"],
        default="new",
        help="Source directory to read from (default: new)"
    )
    parser.add_argument(
        "--bench",
        help="Filter by benchmark name (e.g., 'compare_get')"
    )
    parser.add_argument(
        "--criterion-dir",
        help="Path to criterion directory (default: auto-detect)"
    )
    parser.add_argument(
        "--output",
        default="bench_analysis/benchmark_results.csv",
        help="Output CSV file path (default: bench_analysis/benchmark_results.csv)"
    )
    
    args = parser.parse_args()
    
    # Find criterion directory
    if args.criterion_dir:
        criterion_dir = Path(args.criterion_dir)
    else:
        # Auto-detect from script location
        script_dir = Path(__file__).parent
        repo_root = script_dir.parent
        
        # Try multiple common locations
        possible_locations = [
            repo_root / "target" / "criterion",
            repo_root / "crates" / "massive-graph-core" / "target" / "criterion",
        ]
        
        criterion_dir = None
        for location in possible_locations:
            if location.exists():
                criterion_dir = location
                break
        
        if criterion_dir is None:
            # Default to first location if none exist
            criterion_dir = possible_locations[0]
    
    # Set up output path
    output_path = Path(args.output)
    if not output_path.is_absolute():
        script_dir = Path(__file__).parent
        repo_root = script_dir.parent
        output_path = repo_root / output_path
    
    # Create output directory if needed
    output_path.parent.mkdir(parents=True, exist_ok=True)
    
    # Load existing run IDs to avoid duplicates
    print(f"Checking for existing results in {output_path}...")
    existing_ids = load_existing_run_ids(output_path)
    if existing_ids:
        print(f"Found {len(existing_ids)} existing benchmark records")
    
    # Parse benchmark results
    print(f"Parsing benchmark results from {criterion_dir} (source: {args.source})...")
    if args.tag:
        print(f"Tag: '{args.tag}'")
    
    results = parse_benchmark_results(
        criterion_dir,
        source=args.source,
        bench_filter=args.bench,
        tag=args.tag
    )
    
    if not results:
        print("No benchmark results found.")
        return
    
    print(f"Found {len(results)} benchmark results")
    
    # Export to CSV
    num_exported = export_to_csv(results, output_path, existing_ids)
    
    if num_exported > 0:
        print(f"\n✓ CSV updated successfully: {output_path}")
        print(f"  Total records in CSV: {len(existing_ids)}")


if __name__ == "__main__":
    main()

