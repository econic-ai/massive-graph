use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, black_box};
use massive_graph_core::structures::optimised_index::{OptimisedIndex, Snapshot, MphIndexer};
use massive_graph_core::structures::segmented_stream::SegmentedStream;
use dashmap::DashMap;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

struct IdMph; impl MphIndexer<u64> for IdMph { fn eval(&self, key: &u64) -> usize { *key as usize } }

fn build_index(n: usize) -> OptimisedIndex<u64, u64> {
    let reserved_keys: Arc<[u64]> = Arc::from([]);
    let reserved_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let mph_vals: Arc<[Arc<u64>]> = Arc::from((0..n).map(|i| Arc::new(i as u64)).collect::<Vec<_>>().into_boxed_slice());
    let snap = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: massive_graph_core::structures::optimised_index::ArcIndexer(Arc::new(IdMph)) };
    let delta = Arc::new(SegmentedStream::new());
    OptimisedIndex::new(snap, delta)
}



fn bench_maps_compare(c: &mut Criterion) {
    let mut group = c.benchmark_group("optidx_compare_min");
    let n: usize = 64;

    // HashMap get @ 1024 — setup outside timed loop
    // group.bench_with_input(BenchmarkId::new("hashmap_get", n), &n, |b, &n| {
    //     let m = {
    //         let mut m = HashMap::with_capacity(n);
    //         for i in 0..n as u64 { m.insert(i, i); }
    //         m
    //     };
    //     b.iter(|| { for i in 0..n as u64 { let _ = black_box(m.get(&i)); } });
    // });
    // // DashMap get @ 1024 — setup outside timed loop
    // group.bench_with_input(BenchmarkId::new("dashmap_get", n), &n, |b, &n| {
    //     let m = {
    //         let m = DashMap::with_capacity(n);
    //         for i in 0..n as u64 { m.insert(i, i); }
    //         m
    //     };
    //     b.iter(|| { for i in 0..n as u64 { let _ = black_box(m.get(&i)); } });
    // });
    // // BTreeMap get @ 1024 — setup outside timed loop
    // group.bench_with_input(BenchmarkId::new("btreemap_get", n), &n, |b, &n| {
    //     let m = {
    //         let mut m = BTreeMap::new();
    //         for i in 0..n as u64 { m.insert(i, i); }
    //         m
    //     };
    //     b.iter(|| { for i in 0..n as u64 { let _ = black_box(m.get(&i)); } });
    // });
    // OptimisedIndex get @ 1024 — hoist snapshot outside timed loop
    group.bench_with_input(BenchmarkId::new("optidx_get", n), &n, |b, &n| {
        let idx = {
            let idx = build_index(0);
            for i in 0..n as u64 { idx.upsert(i, i); }
            idx
        };
        let snap = idx.load_snapshot_arc();
        b.iter(|| { for i in 0..n as u64 { let _ = black_box(idx.get_with_snapshot(&snap, &i)); } });
    });

    // OptimisedIndex reserved get @ 1024 — hoist snapshot outside timed loop
    group.bench_with_input(BenchmarkId::new("optidx_get_reserved", n), &n, |b, &n| {
        let idx = {
            let reserved_keys: Arc<[u64]> = Arc::from((0..n as u64).collect::<Vec<_>>().into_boxed_slice());
            let reserved_vals: Arc<[Arc<u64>]> = Arc::from((0..n as u64).map(|i| Arc::new(i)).collect::<Vec<_>>().into_boxed_slice());
            let mph_vals: Arc<[Arc<u64>]> = Arc::from([]);
            let snap = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: massive_graph_core::structures::optimised_index::ArcIndexer(Arc::new(IdMph)) };
            OptimisedIndex::new(snap, Arc::new(SegmentedStream::new()))
        };
        let snap = idx.load_snapshot_arc();
        b.iter(|| { for i in 0..n { let _ = black_box(idx.get_reserved_with_snapshot(&snap, i)); } });
    });

    // OptimisedIndex MPH base get @ 1024 — hoist snapshot outside timed loop
    group.bench_with_input(BenchmarkId::new("optidx_get_mph", n), &n, |b, &n| {
        let idx = build_index(n);
        let snap = idx.load_snapshot_arc();
        b.iter(|| { for i in 0..n as u64 { let _ = black_box(idx.get_with_snapshot(&snap, &i)); } });
    });

    group.finish();
}

criterion_group!(benches,
    bench_maps_compare
);
criterion_main!(benches);


