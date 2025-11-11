/// Diagnostic statistics for RadixIndex performance analysis.

use std::fmt;
use super::arena::ArenaStats;

/// Per-bucket statistics for detailed analysis.
#[derive(Debug, Clone)]
pub struct BucketStats {
    /// Bucket index
    pub bucket_idx: usize,
    /// Number of unique keys (upserts only, excludes tombstones)
    pub key_count: usize,
    /// Total number of records in buffer (includes all upserts and tombstones)
    pub total_records: usize,
    /// Current buffer capacity (slots allocated)
    pub buffer_capacity: usize,
    /// Number of entries in ReadTinyMap
    pub tinymap_size: usize,
    /// Number of times buffer was grown
    pub growth_count: usize,
    /// Tag8 collision count (keys with same tag8)
    pub tag8_collisions: usize,
    /// Unique tag8 values in bucket
    pub unique_tags: usize,
}

/// Comprehensive RadixIndex statistics.
#[derive(Debug, Clone)]
pub struct RadixIndexStats {
    // Index configuration
    /// Total number of buckets (configured)
    pub total_buckets: usize,
    /// Number of bits used for bucket indexing
    pub bucket_bits: u32,
    
    // Usage statistics
    /// Number of buckets actually in use
    pub active_buckets: usize,
    /// Total keys stored across all buckets
    pub total_keys: usize,
    /// Bucket utilization percentage (0.0 - 1.0)
    pub bucket_utilization: f64,
    
    // Distribution metrics
    /// Average keys per active bucket
    pub avg_bucket_depth: f64,
    /// Maximum keys in any bucket
    pub max_bucket_depth: usize,
    /// Minimum keys in active buckets
    pub min_bucket_depth: usize,
    /// Standard deviation of bucket depths
    pub bucket_depth_stddev: f64,
    
    // Hash quality metrics
    /// Shannon entropy of bucket distribution (0.0 - 1.0, higher is better)
    pub bucket_distribution_entropy: f64,
    /// Average tag16 uniqueness ratio per bucket (0.0 - 1.0)
    pub avg_tag16_uniqueness: f64,
    
    // Arena statistics
    /// Unified arena stats for all allocations (Buffer, Recs, TinyMap)
    pub hotpath_arena: ArenaStats,
    /// Deprecated: kept for API compatibility, use hotpath_arena
    pub buffer_arena: ArenaStats,
    /// Deprecated: kept for API compatibility, use hotpath_arena
    pub record_arena: ArenaStats,
    
    // Per-bucket details (optional, can be large)
    /// Detailed stats for each active bucket
    pub bucket_details: Vec<BucketStats>,
}

impl RadixIndexStats {
    /// Calculate derived statistics from bucket details.
    pub fn calculate_derived_stats(&mut self) {
        if self.bucket_details.is_empty() {
            return;
        }
        
        // Calculate depth statistics
        let depths: Vec<usize> = self.bucket_details.iter().map(|b| b.key_count).collect();
        self.total_keys = depths.iter().sum();
        self.active_buckets = depths.iter().filter(|&&d| d > 0).count();
        
        if self.active_buckets > 0 {
            self.avg_bucket_depth = self.total_keys as f64 / self.active_buckets as f64;
            self.max_bucket_depth = *depths.iter().max().unwrap_or(&0);
            self.min_bucket_depth = *depths.iter().filter(|&&d| d > 0).min().unwrap_or(&0);
            
            // Calculate standard deviation
            let variance: f64 = depths.iter()
                .filter(|&&d| d > 0)
                .map(|&d| {
                    let diff = d as f64 - self.avg_bucket_depth;
                    diff * diff
                })
                .sum::<f64>() / self.active_buckets as f64;
            self.bucket_depth_stddev = variance.sqrt();
            
            // Calculate Shannon entropy of bucket distribution
            let total = self.total_keys as f64;
            self.bucket_distribution_entropy = -depths.iter()
                .filter(|&&d| d > 0)
                .map(|&d| {
                    let p = d as f64 / total;
                    p * p.log2()
                })
                .sum::<f64>();
            
            // Normalize entropy to 0-1 range
            let max_entropy = (self.active_buckets as f64).log2();
            if max_entropy > 0.0 {
                self.bucket_distribution_entropy /= max_entropy;
            }
            
            // Calculate average tag16 uniqueness
            self.avg_tag16_uniqueness = self.bucket_details.iter()
                .filter(|b| b.key_count > 0)
                .map(|b| {
                    if b.key_count > 0 {
                        b.unique_tags as f64 / b.key_count as f64
                    } else {
                        0.0
                    }
                })
                .sum::<f64>() / self.active_buckets as f64;
        }
        
        self.bucket_utilization = self.active_buckets as f64 / self.total_buckets as f64;
    }
    
    /// Generate a human-readable summary report.
    pub fn summary_report(&self) -> String {
        format!(
            r#"
RadixIndex Statistics Summary
=============================

Configuration:
  Total Buckets: {}
  Bucket Bits: {}
  Target Entries/Bucket: ~16

Usage:
  Total Keys: {}
  Active Buckets: {} ({:.1}% utilization)
  
Distribution:
  Avg Depth: {:.2} keys/bucket
  Max Depth: {} keys
  Min Depth: {} keys
  Std Dev: {:.2}
  
Hash Quality:
  Bucket Entropy: {:.3} (0=clustered, 1=perfect)
  Tag16 Uniqueness: {:.3} (0=collisions, 1=unique)
  
Arena Memory:
  Total Allocated: {} bytes ({} regions, {} large allocs)
  Large Allocations: {} bytes ({} allocs)
  Retired: {} bytes ({} retires)
"#,
            self.total_buckets,
            self.bucket_bits,
            self.total_keys,
            self.active_buckets,
            self.bucket_utilization * 100.0,
            self.avg_bucket_depth,
            self.max_bucket_depth,
            self.min_bucket_depth,
            self.bucket_depth_stddev,
            self.bucket_distribution_entropy,
            self.avg_tag16_uniqueness,
            self.hotpath_arena.bytes_allocated,
            self.hotpath_arena.num_regions,
            self.hotpath_arena.num_large,
            self.hotpath_arena.bytes_large,
            self.hotpath_arena.num_large,
            self.hotpath_arena.bytes_retired,
            self.hotpath_arena.num_retired,
        )
    }
    
    /// Generate CSV data for bucket distribution analysis.
    pub fn bucket_distribution_csv(&self) -> String {
        let mut csv = String::from("bucket_idx,key_count,total_records,buffer_capacity,tinymap_size,unique_tags,tag8_collisions\n");
        for b in &self.bucket_details {
            if b.key_count > 0 {
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{}\n",
                    b.bucket_idx, b.key_count, b.total_records, b.buffer_capacity, 
                    b.tinymap_size, b.unique_tags, b.tag8_collisions
                ));
            }
        }
        csv
    }
}

impl fmt::Display for BucketStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Bucket[{}]: keys={}, records={}, capacity={}, tinymap={}, unique_tags={}, collisions={}, growths={}",
            self.bucket_idx,
            self.key_count,
            self.total_records,
            self.buffer_capacity,
            self.tinymap_size,
            self.unique_tags,
            self.tag8_collisions,
            self.growth_count
        )
    }
}

impl fmt::Display for RadixIndexStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "RadixIndex Statistics")?;
        writeln!(f, "====================")?;
        writeln!(f)?;
        
        writeln!(f, "Configuration:")?;
        writeln!(f, "  Total Buckets: {}", self.total_buckets)?;
        writeln!(f, "  Bucket Bits: {}", self.bucket_bits)?;
        writeln!(f)?;
        
        writeln!(f, "Usage:")?;
        writeln!(f, "  Total Keys: {}", self.total_keys)?;
        writeln!(f, "  Active Buckets: {} ({:.1}% utilization)", 
                 self.active_buckets, self.bucket_utilization * 100.0)?;
        writeln!(f)?;
        
        writeln!(f, "Distribution:")?;
        writeln!(f, "  Avg Depth: {:.2} keys/bucket", self.avg_bucket_depth)?;
        writeln!(f, "  Max Depth: {} keys", self.max_bucket_depth)?;
        writeln!(f, "  Min Depth: {} keys", self.min_bucket_depth)?;
        writeln!(f, "  Std Dev: {:.2}", self.bucket_depth_stddev)?;
        writeln!(f)?;
        
        writeln!(f, "Hash Quality:")?;
        writeln!(f, "  Bucket Entropy: {:.3} (0=clustered, 1=perfect)", 
                 self.bucket_distribution_entropy)?;
        writeln!(f, "  Tag16 Uniqueness: {:.3} (0=collisions, 1=unique)", 
                 self.avg_tag16_uniqueness)?;
        writeln!(f)?;
        
        writeln!(f, "Arena Memory (hotpath):")?;
        writeln!(f, "  Allocated: {} bytes ({} regions)", 
                 self.hotpath_arena.bytes_allocated, self.hotpath_arena.num_regions)?;
        if self.hotpath_arena.bytes_large > 0 {
            writeln!(f, "  Large Allocations: {} bytes ({} allocs)", 
                     self.hotpath_arena.bytes_large, self.hotpath_arena.num_large)?;
        }
        if self.hotpath_arena.bytes_retired > 0 {
            writeln!(f, "  Retired: {} bytes ({} retires)", 
                     self.hotpath_arena.bytes_retired, self.hotpath_arena.num_retired)?;
        }
        
        if !self.bucket_details.is_empty() {
            writeln!(f)?;
            writeln!(f, "Bucket Details:")?;
            let mut active_buckets: Vec<_> = self.bucket_details.iter()
                .filter(|b| b.key_count > 0)
                .collect();
            active_buckets.sort_by_key(|b| b.bucket_idx);
            
            for bucket in active_buckets {
                writeln!(f, "  {}", bucket)?;
            }
        }
        
        Ok(())
    }
}

