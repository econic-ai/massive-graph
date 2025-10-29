#!/bin/bash

# Script to clear all benchmark cached results
# This removes Criterion's cached benchmark data to force fresh benchmark runs

set -e

# Get the project root directory (parent of bench_analysis)
PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "Clearing benchmark cached results..."
echo "Project root: $PROJECT_ROOT"
echo ""

# Remove Criterion cache directories
if [ -d "$PROJECT_ROOT/target/criterion" ]; then
    echo "✓ Removing target/criterion/"
    rm -rf "$PROJECT_ROOT/target/criterion"
else
    echo "- target/criterion/ not found (already clean)"
fi

# Remove benchmark results in crates/massive-graph-core
if [ -d "$PROJECT_ROOT/crates/massive-graph-core/target/criterion" ]; then
    echo "✓ Removing crates/massive-graph-core/target/criterion/"
    rm -rf "$PROJECT_ROOT/crates/massive-graph-core/target/criterion"
else
    echo "- crates/massive-graph-core/target/criterion/ not found (already clean)"
fi

# Remove benchmark results in server
if [ -d "$PROJECT_ROOT/server/target/criterion" ]; then
    echo "✓ Removing server/target/criterion/"
    rm -rf "$PROJECT_ROOT/server/target/criterion"
else
    echo "- server/target/criterion/ not found (already clean)"
fi

# Remove benchmark results in browser
if [ -d "$PROJECT_ROOT/browser/target/criterion" ]; then
    echo "✓ Removing browser/target/criterion/"
    rm -rf "$PROJECT_ROOT/browser/target/criterion"
else
    echo "- browser/target/criterion/ not found (already clean)"
fi

# Optionally clear generated CSV files (uncomment if desired)
# if [ -f "$PROJECT_ROOT/bench_analysis/benchmark_results.csv" ]; then
#     echo "✓ Removing bench_analysis/benchmark_results.csv"
#     rm -f "$PROJECT_ROOT/bench_analysis/benchmark_results.csv"
# fi

# if [ -f "$PROJECT_ROOT/crates/massive-graph-core/benches/compare_indexes_get_bench.csv" ]; then
#     echo "✓ Removing crates/massive-graph-core/benches/compare_indexes_get_bench.csv"
#     rm -f "$PROJECT_ROOT/crates/massive-graph-core/benches/compare_indexes_get_bench.csv"
# fi

echo ""
echo "✅ Benchmark cache cleared successfully!"
echo "Next benchmark run will start fresh without cached results."


