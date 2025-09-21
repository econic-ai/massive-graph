use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, black_box};
use massive_graph_core::structures::{SegmentedStream, StreamPagePool};
use std::sync::Arc;

fn bench_append_single_thread(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_append_single");
    for &n in &[10_000usize, 100_000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(
                || SegmentedStream::<u64>::new(),
                |s| {
                    for i in 0..n as u64 { let _ = s.append(black_box(i)); }
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_append_multi_writer(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_append_multi");
    for &(writers, per) in &[(4usize, 25_000usize)] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", writers, per)),
            &(writers, per),
            |b, &(writers, per)| {
                b.iter_batched(
                    || Arc::new(SegmentedStream::<u64>::new()),
                    |s| {
                        let mut handles = Vec::with_capacity(writers);
                        for w in 0..writers {
                            let s2 = Arc::clone(&s);
                            handles.push(std::thread::spawn(move || {
                                for i in 0..(per as u64) {
                                    let _ = s2.append(((w as u64) << 32) | i);
                                }
                            }));
                        }
                        for h in handles { let _ = h.join(); }
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

fn bench_iter_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_iter_read");
    for &n in &[100_000usize, 1_000_000] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            // Prepare a stream with n elements
            let s: Arc<SegmentedStream<u64>> = Arc::new(SegmentedStream::new());
            for i in 0..n as u64 { let _ = s.append(i); }

            b.iter(|| {
                let mut c = massive_graph_core::structures::Cursor::new_at_head(&s);
                let mut sum = 0u64;
                while let Some(v) = c.next() { sum = sum.wrapping_add(*v); }
                black_box(sum)
            });
        });
    }
    group.finish();
}

fn bench_append_single_with_prereset_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_append_single_prereset_pool");
    for &n in &[100_000usize] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(
                || {
                    let pool = StreamPagePool::<u64>::with_capacity(12);
                    SegmentedStream::with_pool(pool)
                },
                |s| {
                    for i in 0..n as u64 { let _ = s.append(black_box(i)); }
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_append_with_prefiller(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_append_with_prefiller");
    for &n in &[100_000usize] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter_batched(
                || {
                    let pool = StreamPagePool::<u64>::with_capacity(0).with_prefiller(8);
                    SegmentedStream::with_pool(pool)
                },
                |s| {
                    for i in 0..n as u64 { let _ = s.append(black_box(i)); }
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, bench_append_single_thread, bench_append_multi_writer, bench_iter_read, bench_append_single_with_prereset_pool, bench_append_with_prefiller);
criterion_main!(benches);


