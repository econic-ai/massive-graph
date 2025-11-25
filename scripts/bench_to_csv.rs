// Helper module to add to your benchmarks for direct CSV export
// Usage: Include this in your benchmark file and call `write_results_to_csv` after each bench group

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// Write benchmark results to CSV file
/// Call this after each benchmark group with the collected timing data
pub fn write_results_to_csv(
    csv_path: &str,
    benchmark_name: &str,
    test_name: &str,
    size: usize,
    time_ns: f64,
) -> std::io::Result<()> {
    let path = Path::new(csv_path);
    let file_exists = path.exists();
    
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    
    // Write header if new file
    if !file_exists {
        writeln!(file, "benchmark,test_name,size,time_ns,time_us,time_ms,ops_per_sec")?;
    }
    
    // Calculate derived metrics
    let time_us = time_ns / 1_000.0;
    let time_ms = time_ns / 1_000_000.0;
    let ops_per_sec = if time_ns > 0.0 {
        (size as f64 * 1_000_000_000.0) / time_ns
    } else {
        0.0
    };
    
    writeln!(
        file,
        "{},{},{},{},{},{},{}",
        benchmark_name, test_name, size, time_ns, time_us, time_ms, ops_per_sec
    )?;
    
    Ok(())
}

// Example usage in a benchmark:
/*
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

fn my_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("my_group");
    
    for &n in &[64, 1024, 10000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                // Your benchmark code
                black_box(vec![0u8; n])
            });
        });
    }
    
    group.finish();
}

criterion_group!(benches, my_benchmark);
criterion_main!(benches);
*/

