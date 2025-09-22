use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
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

fn bench_maps_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("optidx_compare_insert_min");
    let n: usize = 64; // match the get-benchmark structure

    // HashMap insert @ 64 — build outside timed loop; measure inserts only
    // group.bench_with_input(BenchmarkId::new("hashmap_insert", n), &n, |b, &n| {
    //     let mut m = HashMap::<u64, u64>::with_capacity(n);
    //     b.iter(|| { for i in 0..n as u64 { m.insert(i, i); }  });
    // });

    // DashMap insert @ 64 — build outside timed loop; measure inserts only
    group.bench_with_input(BenchmarkId::new("dashmap_insert", n), &n, |b, &n| {
        let m = DashMap::<u64, u64>::with_capacity(n);
        b.iter(|| { for i in 0..n as u64 { m.insert(i, i); }  });
    });

    // // BTreeMap insert @ 64 — build outside timed loop; measure inserts only
    // group.bench_with_input(BenchmarkId::new("btreemap_insert", n), &n, |b, &n| {
    //     let mut m = BTreeMap::<u64, u64>::new();
    //     b.iter(|| { for i in 0..n as u64 { m.insert(i, i); }  });
    // });

    // OptimisedIndex insert (upsert) @ 64 — build index outside; precreate Arcs per-sample (not timed), move into upsert (no clone)
    group.bench_with_input(BenchmarkId::new("optidx_insert", n), &n, |b, &n| {
        let idx = build_index(0);
        b.iter(|| { for i in 0..n as u64 { idx.upsert(i, i); } });
    });

    group.finish();
}

criterion_group!(benches, bench_maps_insert);
criterion_main!(benches);


