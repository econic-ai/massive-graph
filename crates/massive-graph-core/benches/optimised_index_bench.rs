use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, black_box};
use massive_graph_core::structures::optimised_index::{OptimisedIndex, Snapshot, MphIndexer, DeltaOp};
use massive_graph_core::structures::segmented_stream::SegmentedStream;
use std::sync::Arc;

struct IdMph; impl MphIndexer<u64> for IdMph { fn eval(&self, key: &u64) -> usize { *key as usize } }

fn build_index(n: usize) -> OptimisedIndex<u64, u64> {
    let reserved_keys: Arc<[u64]> = Arc::from([]);
    let reserved_vals: Arc<[Arc<u64>]> = Arc::from([]);
    let mph_vals: Arc<[Arc<u64>]> = Arc::from((0..n).map(|i| Arc::new(i as u64)).collect::<Vec<_>>().into_boxed_slice());
    let snap = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: massive_graph_core::structures::optimised_index::ArcIndexer(Arc::new(IdMph)) };
    let delta = Arc::new(SegmentedStream::<DeltaOp<u64, u64>>::new());
    OptimisedIndex::new(snap, delta)
}

// fn bench_get_by_index(c: &mut Criterion) {
//     let mut group = c.benchmark_group("optidx_get_by_index");
//     for &n in &[1024usize, 65536] {
//         group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
//             // Build once to isolate by-index get cost
//             let idx = build_index(n);
//             b.iter(|| { for i in 0..n { let _ = black_box(idx.get_by_index(i)); } });
//         });
//     }
//     group.finish();
// }

// fn bench_get_reserved(c: &mut Criterion) {
//     let mut group = c.benchmark_group("optidx_get_reserved");
//     group.bench_function("reserved_8", |b| {
//         let reserved_keys: Arc<[u64]> = Arc::from((0..8u64).collect::<Vec<_>>().into_boxed_slice());
//         let reserved_vals: Arc<[Arc<u64>]> = Arc::from((0..8u64).map(|i| Arc::new(i)).collect::<Vec<_>>().into_boxed_slice());
//         let mph_vals: Arc<[Arc<u64>]> = Arc::from([]);
//         let snap = Snapshot { version: 1, reserved_keys, reserved_vals, mph_vals, mph_indexer: massive_graph_core::structures::optimised_index::ArcIndexer(Arc::new(IdMph)) };
//         let idx: OptimisedIndex<u64, u64> = OptimisedIndex::new(snap, Arc::new(SegmentedStream::new()));
//         b.iter(|| { for i in 0..8 { let _ = black_box(idx.get_reserved_slot(i)); } });
//     });
//     group.finish();
// }

// fn bench_get_key_base(c: &mut Criterion) {
//     let mut group = c.benchmark_group("optidx_get_key_base");
//     for &n in &[1024usize, 65536] {
//         group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
//             // Build once outside the measured closure to isolate get cost
//             let idx = build_index(n);
//             b.iter(|| { for i in 0..n as u64 { let _ = black_box(idx.get(&i)); } });
//         });
//     }
//     group.finish();
// }

fn bench_get_key_delta_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("optidx_get_key_delta_hit");
    for &n in &[1024usize, 65536] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let idx = build_index(n);
            for i in 0..n as u64 { idx.upsert(i, i); }
            b.iter(|| { for i in 0..n as u64 { let _ = black_box(idx.get(&i)); } });
        });
    }
    group.finish();
}

fn bench_get_key_delta_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("optidx_get_key_delta_miss");
    for &n in &[1024usize, 65536] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let idx = build_index(n);
            for i in n as u64..(2*n as u64) { idx.upsert(i, i); }
            b.iter(|| { for i in 0..n as u64 { let _ = black_box(idx.get(&i)); } });
        });
    }
    group.finish();
}

// fn bench_optidx_upsert_delete(c: &mut Criterion) {
//     let mut group = c.benchmark_group("optidx_ops_upsert_delete");
//     for &n in &[1024usize, 65536] {
//         group.bench_with_input(BenchmarkId::new("upsert", n), &n, |b, &n| {
//             b.iter_batched(
//                 || build_index(0),
//                 |idx| { for i in 0..n as u64 { idx.upsert(i, i); } },
//                 BatchSize::SmallInput,
//             );
//         });
//         group.bench_with_input(BenchmarkId::new("delete", n), &n, |b, &n| {
//             b.iter_batched(
//                 || { let idx = build_index(0); for i in 0..n as u64 { idx.upsert(i, i); } idx },
//                 |idx| { for i in 0..n as u64 { idx.remove(&i); } },
//                 BatchSize::SmallInput,
//             );
//         });
//     }
//     group.finish();
// }

// fn bench_optidx_snapshot_rebuild(c: &mut Criterion) {
//     let mut group = c.benchmark_group("optidx_ops_snapshot_rebuild");
//     for &n in &[1024usize, 65536] {
//         group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
//             b.iter_batched(
//                 || build_index(0),
//                 |idx| {
//                     let reserved_keys: Arc<[u64]> = Arc::from([]);
//                     let reserved_vals: Arc<[Arc<u64>]> = Arc::from([]);
//                     let mph_vals: Arc<[Arc<u64>]> = Arc::from((0..n as u64).map(|i| Arc::new(i)).collect::<Vec<_>>().into_boxed_slice());
//                     let snap = Snapshot { version: 2, reserved_keys, reserved_vals, mph_vals, mph_indexer: massive_graph_core::structures::optimised_index::ArcIndexer(Arc::new(IdMph)) };
//                     idx.publish_snapshot(snap);
//                 },
//                 BatchSize::SmallInput,
//             );
//         });
//     }
//     group.finish();
// }

criterion_group!(benches,
    // bench_get_by_index,
    // bench_get_reserved,
    // bench_get_key_base,
    bench_get_key_delta_hit,
    bench_get_key_delta_miss,
    // bench_optidx_upsert_delete,
    // bench_optidx_snapshot_rebuild,
);
criterion_main!(benches);


