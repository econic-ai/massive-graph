use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use massive_graph_core::debug_log;

use massive_graph_core::structures::mph_delta_index::{OptimisedIndex, OptimisedIndexGen, mph_indexer};
use massive_graph_core::structures::mph_delta_index::radix_index_v2::RadixIndexV2;
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


fn build_optidx_mph<V: Clone + std::fmt::Debug + 'static>(keys: &[ID16], vals: &[V], target_capacity: usize) -> OptimisedIndex<ID16, V> {
    // Build MPH indexer
    debug_log!("  Building MPH indexer...");
    
    // Create empty index with appropriate capacity (uses BBHashIndexer by default)
    let max_capacity = target_capacity * 8;
    debug_log!("  Creating OptimisedIndex with capacity {}", max_capacity);
    let idx = OptimisedIndex::new_with_capacity(target_capacity, max_capacity);
    debug_log!("  OptimisedIndex created successfully");
    
    // Insert all key-value pairs into radix index
    for (k, v) in keys.iter().zip(vals.iter().cloned()) {
        idx.upsert(k.clone(), v);
    }
    eprintln!("  OptimisedIndex MPH built successfully, Size is {}", idx.len());
    // Publish to build the MPH index
    idx.publish();
    eprintln!("  OptimisedIndex MPH published successfully, Size is {}", idx.len());
    idx
}

#[allow(dead_code)]
fn build_optidx_mph_from_keys<V: Clone + std::fmt::Debug + 'static>(keys: &[ID16], vals: &[V], target_capacity: usize) -> OptimisedIndex<ID16, V> {
    let mph = mph_indexer::BBHashIndexer::build(&keys, Default::default());
    let idx = OptimisedIndex::new_with_base_keys_and_capacity(
        &keys,
        vals.to_vec(),
        mph,
        target_capacity
    );
    idx
}

#[allow(dead_code)]
#[derive(Clone)]
struct ZeroMph;
impl mph_indexer::MphIndexer<ID16> for ZeroMph { 
    fn eval(&self, _key: &ID16) -> usize { 0 } 
    fn build(_keys: &[ID16]) -> Self { ZeroMph }
}

#[allow(dead_code)]
fn build_optidx_delta<V: Clone + std::fmt::Debug + 'static>(keys: &[ID16], vals: &[V], target_capacity: usize) -> OptimisedIndexGen<ID16, V, ZeroMph> {
    // Empty base; all inserts go to new-keys delta (radix index)
    let max_capacity = target_capacity * 2;
    let idx = OptimisedIndexGen::new_with_indexer_and_capacity(ZeroMph, target_capacity, max_capacity);
    for (k, v) in keys.iter().zip(vals.iter().cloned()) { idx.upsert(k.clone(), v); }
    idx.consolidate_radix_only();
    idx
}

fn build_radix_v2<V: Copy + std::fmt::Debug + 'static>(keys: &[ID16], vals: &[V], target_capacity: usize) -> RadixIndexV2<ID16, V> {
    let max_capacity = target_capacity * 2;
    let idx = RadixIndexV2::with_capacity(target_capacity, max_capacity);
    for (k, v) in keys.iter().zip(vals.iter()) {
        idx.upsert(k, v);
    }
    idx
}

fn bench_variant<V: Clone + Copy + std::fmt::Debug + 'static>(c: &mut Criterion, label: &'static str, make: fn(usize)->V) {
    let sizes: &[usize] = &[64, 1024, 10000, 65536];
    // let sizes: &[usize] = &[1000000];

    let mut group = c.benchmark_group(format!("compare_get/{label}"));
    for &n in sizes {
        eprintln!("=== Building test data for n={} ===", n);
        
        // Generate keys and values independently
        let keys: Vec<ID16> = (0..n).map(|_| ID16::random()).collect();
        let vals: Vec<V> = build_vals(n, make);

        
        // eprintln!("  Building OptimisedIndex MPH...");
        // // Build indexes with capacity matching the benchmark size
        // let oi_mph = build_optidx_mph_from_keys(&keys, &vals, n);
        // eprintln!("  OptimisedIndex MPH built successfully. Size is {}", oi_mph.len());

        // eprintln!("  Building OptimisedIndex Radix...");
        // let oi_radix = build_optidx_delta(&keys, &vals, n);
        
        eprintln!("  Building RadixIndexV2...");
        let radix_v2 = build_radix_v2(&keys, &vals, n);
        eprintln!("  RadixIndexV2 built successfully. Size check: {} keys", keys.len());
        
        // let mut hm: HashMap<ID16, V> = HashMap::with_capacity(n);
        // for i in 0..n { hm.insert(keys[i].clone(), vals[i].clone()); }
        // let dm: DashMap<ID16, V> = {
        //     let d = DashMap::with_capacity(n);
        //     for i in 0..n { d.insert(keys[i].clone(), hm.get(&keys[i]).unwrap().clone()); }
        //     d
        // };
        
        // let mut bt: BTreeMap<ID16, V> = BTreeMap::new();
        // for i in 0..n { bt.insert(keys[i].clone(), hm.get(&keys[i]).unwrap().clone()); }
        
        
        // group.bench_with_input(BenchmarkId::new("hashmap/get", n), &n, |b, &_n| {
        //     b.iter(|| { let mut acc=0usize; for i in 0..n { acc ^= black_box(hm.get(&keys[i]).map(|v| core::mem::size_of_val(v)).unwrap_or(0)); } black_box(acc); });
        // });
        // group.bench_with_input(BenchmarkId::new("dashmap/get", n), &n, |b, &_n| {
        //     b.iter(|| { let mut acc=0usize; for i in 0..n { acc ^= black_box(dm.get(&keys[i]).as_deref().map(|v| core::mem::size_of_val(v)).unwrap_or(0)); } black_box(acc); });
        // });
        // group.bench_with_input(BenchmarkId::new("btree/get", n), &n, |b, &_n| {
        //     b.iter(|| { let mut acc=0usize; for i in 0..n { acc ^= black_box(bt.get(&keys[i]).map(|v| core::mem::size_of_val(v)).unwrap_or(0)); } black_box(acc); });
        // });
        // group.bench_with_input(BenchmarkId::new("optidx_radix/get", n), &n, |b, &_n| {
        //     let radix_get_guard = crossbeam_epoch::pin();             
        //     b.iter(|| { 
        //         let mut acc=0usize;
        //         for i in 0..n {
        //             acc ^= black_box(oi_radix.get_radix(&keys[i], &radix_get_guard).map(|_| 0).unwrap_or(0));
        //         }
        //         black_box(acc);
        //     });
        // });

        group.bench_with_input(BenchmarkId::new("radix_v2/get", n), &n, |b, &_n| {
            b.iter(|| { 
                let mut acc=0usize;
                for i in 0..n {
                    acc ^= black_box(radix_v2.get(&keys[i]).map(|_| 0).unwrap_or(0));
                }
                black_box(acc);
            });
        });

        // OptimisedIndex: get from MPH index only
        // group.bench_with_input(BenchmarkId::new("optidx_mph/get", n), &n, |b, &_n| {
        //     let mph_guard = crossbeam_epoch::pin();
        //     let snapshot = oi_mph.snapshot(&mph_guard);
        //     b.iter(|| { 
        //         let mut acc=0usize; 
        //         for i in 0..n { 
        //             acc ^= black_box(oi_mph.get_mph_from_snapshot(snapshot, &keys[i]).map(|_v| 0).unwrap_or(0)); 
        //         } 
        //         black_box(acc); 
        //     });
        // });

        // OptimisedIndex: standard get only
        // group.bench_with_input(BenchmarkId::new("optidx_standard_mph_heavy/get", n), &n, |b, &_n| {
        //     let opt_mph_get_guard = crossbeam_epoch::pin();
        //     b.iter(|| { 
        //         let mut acc=0usize; 
        //         for i in 0..n { 
        //             acc ^= black_box(oi_mph.get(&keys[i], &opt_mph_get_guard).map(|_v| 0).unwrap_or(0)); 
        //         } 
        //         black_box(acc); 
        //     });
        // });        
        
        // // OptimisedIndex: standard get only
        // group.bench_with_input(BenchmarkId::new("optidx_standard_delta_heavy/get", n), &n, |b, &_n| {
        //     let opt_delta_get_guard = crossbeam_epoch::pin();
        //     b.iter(|| { 
        //         let mut acc=0usize; 
        //         for i in 0..n { 
        //             acc ^= black_box(oi_radix.get(&keys[i], &opt_delta_get_guard).map(|_v| 0).unwrap_or(0)); 
        //         } 
        //         black_box(acc); 
        //     });
        // });        
                
        
        // // group.bench_with_input(BenchmarkId::new("hashmap/iter_all", n), &n, |b, &_n| {
        // //     let mut acc_hm = 0usize;
        // //     b.iter(|| {
        // //         let mut acc=0usize;
        // //         for _r in 0..10 { for (_k, v) in hm.iter() { acc ^= black_box(core::mem::size_of_val(v)); } }
        // //         black_box(acc);
        // //     });
        // // });

        // // group.bench_with_input(BenchmarkId::new("dashmap/iter_all", n), &n, |b, &_n| {
        // //     let mut acc_dm = 0usize;
        // //     b.iter(|| {
        // //         let mut acc=0usize;
        // //         for _r in 0..10 { for r in dm.iter() { acc ^= black_box(core::mem::size_of_val(r.value())); } }
        // //         black_box(acc);
        // //     });
        // // });
        // // group.bench_with_input(BenchmarkId::new("btree/iter_all", n), &n, |b, &_n| {
        // //     let mut acc_bt = 0usize;

        // //     b.iter(|| {
        // //         let mut acc=0usize;
        // //         for (_k, v) in bt.iter() {
        // //             acc ^= black_box(core::mem::size_of_val(v));
        // //         }
        // //         black_box(acc);
        // //     });
        // // });
        // group.bench_with_input(BenchmarkId::new("optidx_radix/iter_all", n), &n, |b, &_n| {
        //     let guard_all = crossbeam_epoch::pin();
        //     let mut acc_all = 0usize;            
        //     b.iter(|| {
        //         for _ in oi_radix.iter_radix(&guard_all) {
        //             acc_all ^= black_box(0);
        //         }
        //         black_box(&acc_all);
        //     });
        // });

        // group.bench_with_input(BenchmarkId::new("optidx_mph/iter_all", n), &n, |b, &_n| {
        //     let guard_all = crossbeam_epoch::pin();
        //     let snapshot = oi_mph.snapshot(&guard_all);
        //     let mut acc_all = 0usize;   
        //     // eprintln!("  Iterating over MPH index...size is {}", snapshot.len());         
        //     b.iter(|| {
        //         for _v in oi_mph.iter_mph_from_snapshot(snapshot) {
        //             acc_all ^= black_box(0);
        //         }
        //         black_box(&acc_all);
        //     });
        // });
      
    }
    group.finish();
}

pub fn compare_get_benchmarks(c: &mut Criterion) {
    bench_variant::<V16>(c, "16b", make_v16);
    // bench_variant::<V32>(c, "32b", make_v32);
    // bench_variant::<V128>(c, "128b", make_v128);
}

criterion_group!(benches, compare_get_benchmarks);
criterion_main!(benches);


