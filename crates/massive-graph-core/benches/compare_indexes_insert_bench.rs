use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use massive_graph_core::structures::optimised_index::{OptimisedIndex, Snapshot, MphIndexer};
use massive_graph_core::structures::segmented_stream::SegmentedStream;
use massive_graph_core::types::{ID8, ID16, ID32};
use dashmap::DashMap;
use std::collections::{HashMap};
use std::sync::Arc;

struct IdMph;
impl<K> MphIndexer<K> for IdMph { fn eval(&self, _key: &K) -> usize { 0 } }

fn build_index<K: Clone + Eq + std::hash::Hash + 'static, V: Clone + 'static>(n: usize) -> OptimisedIndex<K, V> {
    let reserved_keys: Arc<[K]> = Arc::from([] as [K; 0]);
    let reserved_vals: Arc<[Arc<V>]> = Arc::from([]);
    let mph_vals: Arc<[Arc<V>]> = Arc::from(Vec::<Arc<V>>::with_capacity(n).into_boxed_slice());
    let snap: Snapshot<K, V> = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: massive_graph_core::structures::optimised_index::ArcIndexer(Arc::new(IdMph)) };
    let delta = Arc::new(SegmentedStream::new());
    OptimisedIndex::new(snap, delta)
}

fn bench_maps_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("optidx_compare_insert_min");
    for &n in &[64, 1024, 65536] {
        let keys8: Vec<ID8> = (0..n).map(|_| ID8::random()).collect();
        let keys16: Vec<ID16> = (0..n).map(|_| ID16::random()).collect();
        let keys32: Vec<ID32> = (0..n).map(|_| ID32::random()).collect();
        let vals_small: Vec<u64> = (0..n as u64).map(|i| i).collect();

        // small V with ID8
        group.bench_with_input(BenchmarkId::new("hashmap_u64/ID8", n), &n, |b, &n| {
            let mut m = HashMap::<ID8, u64>::with_capacity(n);
            b.iter(|| { for i in 0..n { m.insert(keys8[i], vals_small[i]); }  });
        });
        group.bench_with_input(BenchmarkId::new("dashmap_u64/ID8", n), &n, |b, &n| {
            let m = DashMap::<ID8, u64>::with_capacity(n);
            b.iter(|| { for i in 0..n { m.insert(keys8[i], vals_small[i]); }  });
        });
        group.bench_with_input(BenchmarkId::new("optidx_u64/ID8", n), &n, |b, &n| {
            let idx: OptimisedIndex<ID8, u64> = build_index(0);
            b.iter(|| { for i in 0..n { idx.upsert(keys8[i], vals_small[i]); } });
        });

        // small V with ID16
        group.bench_with_input(BenchmarkId::new("hashmap_u64/ID16", n), &n, |b, &n| {
            let mut m = HashMap::<ID16, u64>::with_capacity(n);
            b.iter(|| { for i in 0..n { m.insert(keys16[i], vals_small[i]); }  });
        });
        group.bench_with_input(BenchmarkId::new("dashmap_u64/ID16", n), &n, |b, &n| {
            let m = DashMap::<ID16, u64>::with_capacity(n);
            b.iter(|| { for i in 0..n { m.insert(keys16[i], vals_small[i]); }  });
        });
        group.bench_with_input(BenchmarkId::new("optidx_u64/ID16", n), &n, |b, &n| {
            let idx: OptimisedIndex<ID16, u64> = build_index(0);
            b.iter(|| { for i in 0..n { idx.upsert(keys16[i], vals_small[i]); } });
        });

        // small V with ID32
        group.bench_with_input(BenchmarkId::new("hashmap_u64/ID32", n), &n, |b, &n| {
            let mut m = HashMap::<ID32, u64>::with_capacity(n);
            b.iter(|| { for i in 0..n { m.insert(keys32[i], vals_small[i]); }  });
        });
        group.bench_with_input(BenchmarkId::new("dashmap_u64/ID32", n), &n, |b, &n| {
            let m = DashMap::<ID32, u64>::with_capacity(n);
            b.iter(|| { for i in 0..n { m.insert(keys32[i], vals_small[i]); }  });
        });
        group.bench_with_input(BenchmarkId::new("optidx_u64/ID32", n), &n, |b, &n| {
            let idx: OptimisedIndex<ID32, u64> = build_index(0);
            b.iter(|| { for i in 0..n { idx.upsert(keys32[i], vals_small[i]); } });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_maps_insert);
criterion_main!(benches);


