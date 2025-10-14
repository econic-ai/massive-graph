use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::{BTreeMap, HashMap};
use dashmap::DashMap;
use std::sync::Arc;

use massive_graph_core::structures::mph_delta_index::{OptimisedIndexGen, mph_indexer};
use massive_graph_core::types::ids::ID16;

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct V16([u8; 16]);
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct V32([u8; 32]);
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
struct V128([u8; 128]);

fn build_vals<V: Clone>(n: usize, make: fn(usize)->V) -> Vec<V> { (0..n).map(|i| make(i)).collect() }
fn make_v16(i: usize) -> V16 { let mut b=[0u8;16]; b[0]=(i&0xFF) as u8; b[15]=((i>>8)&0xFF) as u8; V16(b) }
#[allow(dead_code)]
fn make_v32(i: usize) -> V32 { let mut b=[0u8;32]; b[0]=(i&0xFF) as u8; b[31]=((i>>8)&0xFF) as u8; V32(b) }
#[allow(dead_code)]
fn make_v128(i: usize) -> V128 { let mut b=[0u8;128]; b[0]=(i&0xFF) as u8; b[127]=((i>>8)&0xFF) as u8; V128(b) }

#[allow(dead_code)]
#[derive(Clone)]
struct ZeroMph;
impl mph_indexer::MphIndexer<ID16> for ZeroMph { 
    fn eval(&self, _key: &ID16) -> usize { 0 } 
    fn build(_keys: &[ID16]) -> Self { ZeroMph }
}

// #[allow(dead_code)]
fn build_optidx_delta<V: Clone + std::fmt::Debug + 'static>(keys: &[ID16], vals: &[V], target_capacity: usize) -> OptimisedIndexGen<ID16, V, ZeroMph> {
    // Empty base; all inserts go to new-keys delta (radix index)
    let max_capacity = target_capacity * 8;
    let idx = OptimisedIndexGen::new_with_indexer_and_capacity(ZeroMph, target_capacity, max_capacity);
    for (k, v) in keys.iter().zip(vals.iter().cloned()) { idx.upsert(k.clone(), v); }
    idx
}

fn bench_variant<V: Clone + std::fmt::Debug + 'static>(c: &mut Criterion, label: &'static str, make: fn(usize)->V) {
    let sizes: &[usize] = &[64, 1024, 10000, 65536];

    let mut group = c.benchmark_group(format!("compare_insert/{label}"));
    for &n in sizes {
        eprintln!("=== Building test data for n={} ===", n);
        
        // Generate keys and values independently (outside benchmark loop)
        let keys: Vec<ID16> = (0..n).map(|_| ID16::random()).collect();
        eprintln!("  Built {} keys", keys.len());

        let vals: Vec<V> = build_vals(n, make);
        eprintln!("  Built {} values", vals.len());

        // let update_vals: Vec<V> = (0..n).map(|i| make(i + 1000000)).collect();
        // eprintln!("  Built {} update values", update_vals.len());

        // ============================================================
        // INSERT BENCHMARKS (empty -> full)
        // ============================================================
        
        // group.bench_with_input(BenchmarkId::new("hashmap/insert", n), &n, |b, &_n| {
        //     let mut hm = HashMap::with_capacity(n);
        //     b.iter(|| {
        //         for i in 0..n {
        //             black_box(hm.insert(keys[i].clone(), vals[i].clone()));
        //         }
        //     });
        // });

        // group.bench_with_input(BenchmarkId::new("dashmap/insert", n), &n, |b, &_n| {
        //     let dm = DashMap::with_capacity(n);
        //     b.iter(|| {
        //         for i in 0..n {
        //             black_box(dm.insert(keys[i].clone(), vals[i].clone()));
        //         }
        //     });
        // });

        // group.bench_with_input(BenchmarkId::new("btree/insert", n), &n, |b, &_n| {
        //     let mut bt = BTreeMap::new();
        //     b.iter(|| {
        //         for i in 0..n {
        //             black_box(bt.insert(keys[i].clone(), vals[i].clone()));
        //         }
        //     });
        // });

        group.bench_with_input(BenchmarkId::new("optidx_radix/insert", n), &n, |b, &_n| {
            let max_capacity = n * 4;
            let idx = OptimisedIndexGen::new_with_indexer_and_capacity(ZeroMph, n, max_capacity);
            b.iter(|| {
                    for i in 0..n {
                        black_box(idx.upsert(keys[i].clone(), vals[i].clone()));
                    }   
                }
            );
            // let guard = crossbeam_epoch::pin();
            // let radix_stats = idx.radix_stats(&guard);
            // eprintln!("{}", radix_stats.summary_report());
        });        
        
        // Collect and display RadixIndex stats after benchmark
        // eprintln!("\n=== RadixIndex Stats for n={} ===", n);
        // let test_idx = build_optidx_delta(&keys, &vals, n);
        // let guard = crossbeam_epoch::pin();
        // let radix_stats = test_idx.radix_stats(&guard);
        // eprintln!("{}", radix_stats.summary_report());

        // ============================================================
        // UPDATE BENCHMARKS (existing keys) - setup outside, update inside
        // ============================================================
        
        // // Pre-populate structures outside benchmark loop
        // let mut hm_update: HashMap<ID16, V> = HashMap::with_capacity(n);
        // for i in 0..n { hm_update.insert(keys[i].clone(), vals[i].clone()); }
        
        // group.bench_with_input(BenchmarkId::new("hashmap/update", n), &n, |b, &_n| {
        //     b.iter(|| {
        //         for i in 0..n {
        //             black_box(hm_update.insert(keys[i].clone(), update_vals[i].clone()));
        //         }
        //     });
        // });

        // let dm_update: DashMap<ID16, V> = DashMap::with_capacity(n);
        // for i in 0..n { dm_update.insert(keys[i].clone(), vals[i].clone()); }
        
        // group.bench_with_input(BenchmarkId::new("dashmap/update", n), &n, |b, &_n| {
        //     b.iter(|| {
        //         for i in 0..n {
        //             black_box(dm_update.insert(keys[i].clone(), update_vals[i].clone()));
        //         }
        //     });
        // });

        // let mut bt_update: BTreeMap<ID16, V> = BTreeMap::new();
        // for i in 0..n { bt_update.insert(keys[i].clone(), vals[i].clone()); }
        
        // group.bench_with_input(BenchmarkId::new("btree/update", n), &n, |b, &_n| {
        //     b.iter(|| {
        //         for i in 0..n {
        //             black_box(bt_update.insert(keys[i].clone(), update_vals[i].clone()));
        //         }
        //     });
        // });

        // let oi_radix_update: OptimisedIndex<ID16, V> = build_optidx_delta(&keys, &vals, n);
        
        // group.bench_with_input(BenchmarkId::new("optidx_radix/update", n), &n, |b, &_n| {
        //     b.iter(|| {
        //         for i in 0..n {
        //             black_box(oi_radix_update.upsert(keys[i].clone(), update_vals[i].clone()));
        //         }
        //     });
        // });

        // ============================================================
        // DELETE BENCHMARKS (remove all keys) - setup outside, delete inside
        // ============================================================
        
        // let mut hm_delete: HashMap<ID16, V> = HashMap::with_capacity(n);
        // for i in 0..n { hm_delete.insert(keys[i].clone(), vals[i].clone()); }
        
        // group.bench_with_input(BenchmarkId::new("hashmap/delete", n), &n, |b, &_n| {
        //     b.iter(|| {
        //         for i in 0..n {
        //             black_box(hm_delete.remove(&keys[i]));
        //         }
        //     });
        // });

        // let dm_delete: DashMap<ID16, V> = DashMap::with_capacity(n);
        // for i in 0..n { dm_delete.insert(keys[i].clone(), vals[i].clone()); }
        
        // group.bench_with_input(BenchmarkId::new("dashmap/delete", n), &n, |b, &_n| {
        //     b.iter(|| {
        //         for i in 0..n {
        //             black_box(dm_delete.remove(&keys[i]));
        //         }
        //     });
        // });

        // let mut bt_delete: BTreeMap<ID16, V> = BTreeMap::new();
        // for i in 0..n { bt_delete.insert(keys[i].clone(), vals[i].clone()); }
        
        // group.bench_with_input(BenchmarkId::new("btree/delete", n), &n, |b, &_n| {
        //     b.iter(|| {
        //         for i in 0..n {
        //             black_box(bt_delete.remove(&keys[i]));
        //         }
        //     });
        // });

        // let oi_radix_delete: OptimisedIndex<ID16, V> = build_optidx_delta(&keys, &vals, n);
        
        // group.bench_with_input(BenchmarkId::new("optidx_radix/delete", n), &n, |b, &_n| {
        //     b.iter(|| {
        //         for i in 0..n {
        //             black_box(oi_radix_delete.remove(&keys[i]));
        //         }
        //     });
        // });
    }
    group.finish();
}

pub fn compare_insert_benchmarks(c: &mut Criterion) {
    bench_variant::<V16>(c, "16b", make_v16);
    // bench_variant::<V32>(c, "32b", make_v32);
    // bench_variant::<V128>(c, "128b", make_v128);
}

criterion_group!(benches, compare_insert_benchmarks);
criterion_main!(benches);
