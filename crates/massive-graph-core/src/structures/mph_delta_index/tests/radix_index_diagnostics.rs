/// Diagnostic tests for RadixIndex performance analysis at various scales.
/// 
/// These tests collect detailed statistics about the index structure to identify
/// performance bottlenecks and hash quality issues.

use massive_graph_core::structures::mph_delta_index::radix_index::RadixIndex;
use massive_graph_core::structures::segmented_stream::segmented_stream::SegmentedStream;
use massive_graph_core::types::ID16;
use crossbeam_epoch as epoch;

/// Helper to create test keys
fn make_key(i: u64) -> ID16 {
    let bytes = i.to_le_bytes();
    let mut full_bytes = [0u8; 16];
    full_bytes[..8].copy_from_slice(&bytes);
    ID16::from_bytes(full_bytes)
}

/// Helper to populate index with sequential keys
fn populate_index(count: usize, stream: &SegmentedStream<u64>) -> RadixIndex<ID16, u64> {
    println!("  [populate_index] Creating index with capacity {}", count);
    let idx = RadixIndex::with_capacity(count, count * 4, stream);
    println!("  [populate_index] Index created, starting upserts...");
    let guard = epoch::pin();
    
    let checkpoint = count / 10;
    for i in 0..count {
        if checkpoint > 0 && i % checkpoint == 0 {
            println!("  [populate_index] Progress: {}/{} ({:.0}%)", i, count, (i as f64 / count as f64) * 100.0);
        }
        if i >= 900 && i < 950 {
            println!("  [populate_index] Upsert #{}", i);
        }
        let key = make_key(i as u64);
        let val = (i * 10) as u64;
        let sidx = stream.append_with_index(val).expect("append failed");
        idx.upsert(&key, &sidx, &guard);
        if i >= 900 && i < 950 {
            println!("  [populate_index] Upsert #{} complete", i);
        }
    }
    println!("  [populate_index] All {} upserts complete", count);
    idx
}

#[test]
#[ignore] // Run explicitly with: cargo test --release radix_diagnostics -- --ignored --nocapture
fn radix_diagnostics_scale_64() {
    println!("\n{}", "=".repeat(80));
    println!("RADIX INDEX DIAGNOSTICS - Scale 64");
    println!("{}\n", "=".repeat(80));
    
    println!("[scale_64] Creating stream...");
    let stream = SegmentedStream::<u64>::new();
    println!("[scale_64] Populating index...");
    let idx = populate_index(64, &stream);
    
    println!("[scale_64] Collecting stats...");
    let guard = epoch::pin();
    let stats = idx.collect_stats(&guard);
    
    println!("[scale_64] Generating report...");
    println!("{}", stats.summary_report());
    println!("[scale_64] COMPLETE");
    println!("\nBucket Distribution (first 20):");
    for bucket in stats.bucket_details.iter().take(20) {
        println!("  Bucket {}: {} keys, {} capacity, {} tinymap, {} unique tags",
            bucket.bucket_idx, bucket.key_count, bucket.buffer_capacity,
            bucket.tinymap_size, bucket.unique_tags);
    }
}

#[test]
#[ignore]
fn radix_diagnostics_scale_1024() {
    println!("\n{}", "=".repeat(80));
    println!("RADIX INDEX DIAGNOSTICS - Scale 1024");
    println!("{}\n", "=".repeat(80));
    
    println!("[scale_1024] Creating stream...");
    let stream = SegmentedStream::<u64>::new();
    println!("[scale_1024] Populating index...");
    let idx = populate_index(1024, &stream);
    
    println!("[scale_1024] Collecting stats...");
    let guard = epoch::pin();
    let stats = idx.collect_stats(&guard);
    
    println!("[scale_1024] Generating report...");
    println!("{}", stats.summary_report());
    println!("[scale_1024] COMPLETE");
    
    // Analyze distribution
    let depths: Vec<usize> = stats.bucket_details.iter().map(|b| b.key_count).collect();
    let histogram = create_histogram(&depths);
    println!("\nBucket Depth Histogram:");
    for (depth, count) in histogram {
        println!("  {} keys: {} buckets", depth, count);
    }
}

#[test]
#[ignore]
fn radix_diagnostics_scale_10000() {
    println!("\n{}", "=".repeat(80));
    println!("RADIX INDEX DIAGNOSTICS - Scale 10,000");
    println!("{}\n", "=".repeat(80));
    
    println!("[scale_10000] Creating stream...");
    let stream = SegmentedStream::<u64>::new();
    println!("[scale_10000] Populating index...");
    let idx = populate_index(10000, &stream);
    
    println!("[scale_10000] Collecting stats...");
    let guard = epoch::pin();
    let stats = idx.collect_stats(&guard);
    
    println!("[scale_10000] Generating report...");
    println!("{}", stats.summary_report());
    println!("[scale_10000] COMPLETE");
    
    // Analyze distribution
    let depths: Vec<usize> = stats.bucket_details.iter().map(|b| b.key_count).collect();
    let histogram = create_histogram(&depths);
    println!("\nBucket Depth Histogram:");
    for (depth, count) in histogram {
        println!("  {} keys: {} buckets", depth, count);
    }
    
    // Check for outliers
    let outliers: Vec<_> = stats.bucket_details.iter()
        .filter(|b| b.key_count > (stats.avg_bucket_depth * 2.0) as usize)
        .collect();
    
    if !outliers.is_empty() {
        println!("\n⚠️  Outlier Buckets (>2x avg depth):");
        for bucket in outliers {
            println!("  Bucket {}: {} keys ({:.1}x avg), {} unique tags ({:.1}% uniqueness)",
                bucket.bucket_idx, bucket.key_count,
                bucket.key_count as f64 / stats.avg_bucket_depth,
                bucket.unique_tags,
                (bucket.unique_tags as f64 / bucket.key_count as f64) * 100.0);
        }
    }
}

#[test]
#[ignore]
fn radix_diagnostics_scale_65536() {
    println!("\n{}", "=".repeat(80));
    println!("RADIX INDEX DIAGNOSTICS - Scale 65,536 (CRITICAL)");
    println!("{}\n", "=".repeat(80));
    
    println!("[scale_65536] Creating stream...");
    let stream = SegmentedStream::<u64>::new();
    println!("[scale_65536] Populating index...");
    let idx = populate_index(65536, &stream);
    
    println!("[scale_65536] Collecting stats...");
    let guard = epoch::pin();
    let stats = idx.collect_stats(&guard);
    
    println!("[scale_65536] Generating report...");
    println!("{}", stats.summary_report());
    println!("[scale_65536] COMPLETE");
    
    // Detailed analysis for the problematic scale
    let depths: Vec<usize> = stats.bucket_details.iter().map(|b| b.key_count).collect();
    let histogram = create_histogram(&depths);
    println!("\nBucket Depth Histogram:");
    for (depth, count) in histogram {
        println!("  {} keys: {} buckets", depth, count);
    }
    
    // Check for severe outliers
    let severe_outliers: Vec<_> = stats.bucket_details.iter()
        .filter(|b| b.key_count > (stats.avg_bucket_depth * 3.0) as usize)
        .collect();
    
    if !severe_outliers.is_empty() {
        println!("\n🚨 SEVERE Outlier Buckets (>3x avg depth):");
        for bucket in severe_outliers {
            println!("  Bucket {}: {} keys ({:.1}x avg), {} unique tags ({:.1}% uniqueness)",
                bucket.bucket_idx, bucket.key_count,
                bucket.key_count as f64 / stats.avg_bucket_depth,
                bucket.unique_tags,
                (bucket.unique_tags as f64 / bucket.key_count as f64) * 100.0);
        }
    }
    
    // Tag16 collision analysis
    let high_collision_buckets: Vec<_> = stats.bucket_details.iter()
        .filter(|b| b.tag16_collisions > 5)
        .collect();
    
    if !high_collision_buckets.is_empty() {
        println!("\n⚠️  High Tag16 Collision Buckets (>5 collisions):");
        for bucket in high_collision_buckets.iter().take(10) {
            println!("  Bucket {}: {} collisions, {} keys, {} unique tags",
                bucket.bucket_idx, bucket.tag16_collisions, 
                bucket.key_count, bucket.unique_tags);
        }
    }
    
    // Arena memory analysis
    println!("\n📊 Memory Efficiency:");
    let total_keys = stats.total_keys;
    let total_arena_bytes = stats.hotpath_arena.bytes_allocated 
        + stats.buffer_arena.bytes_allocated 
        + stats.record_arena.bytes_allocated;
    println!("  Total arena memory: {} bytes", total_arena_bytes);
    println!("  Bytes per key: {:.1}", total_arena_bytes as f64 / total_keys as f64);
    println!("  Hotpath retirements: {} ({:.1}% of allocs)",
        stats.hotpath_arena.retire_count,
        (stats.hotpath_arena.retire_count as f64 / stats.hotpath_arena.alloc_count as f64) * 100.0);
}

#[test]
#[ignore]
fn radix_diagnostics_hash_quality() {
    println!("\n{}", "=".repeat(80));
    println!("HASH QUALITY ANALYSIS - All Scales");
    println!("{}\n", "=".repeat(80));
    
    let scales = vec![64, 1024, 10000, 65536];
    
    println!("{:<10} {:<15} {:<15} {:<15} {:<15}", 
        "Scale", "Bucket Entropy", "Tag16 Unique", "Avg Depth", "Max Depth");
    println!("{}", "-".repeat(70));
    
    for &scale in &scales {
        let stream = SegmentedStream::<u64>::new();
        let idx = populate_index(scale, &stream);
        
        let guard = epoch::pin();
        let stats = idx.collect_stats(&guard);
        
        println!("{:<10} {:<15.3} {:<15.3} {:<15.2} {:<15}",
            scale,
            stats.bucket_distribution_entropy,
            stats.avg_tag16_uniqueness,
            stats.avg_bucket_depth,
            stats.max_bucket_depth);
    }
    
    println!("\nInterpretation:");
    println!("  Bucket Entropy: 1.0 = perfect distribution, <0.8 = clustering");
    println!("  Tag16 Unique:   1.0 = no collisions, <0.9 = high collision rate");
    println!("  Avg Depth:      Should be ~16 for optimal performance");
    println!("  Max Depth:      Should be <32 to avoid buffer growth");
}

/// Create a histogram of bucket depths
fn create_histogram(depths: &[usize]) -> Vec<(usize, usize)> {
    let mut counts = std::collections::HashMap::new();
    for &depth in depths {
        if depth > 0 {
            *counts.entry(depth).or_insert(0) += 1;
        }
    }
    
    let mut histogram: Vec<_> = counts.into_iter().collect();
    histogram.sort_by_key(|&(depth, _)| depth);
    histogram
}

