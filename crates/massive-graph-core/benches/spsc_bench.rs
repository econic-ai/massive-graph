use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, black_box};
use massive_graph_core::structures::spsc::spsc::SpscRing;
use std::sync::Arc;

fn bench_spsc_producer_only(c: &mut Criterion) {
	let mut group = c.benchmark_group("spsc_producer_only");
	for &cap in &[64usize, 256, 4096] {
		group.bench_with_input(BenchmarkId::from_parameter(cap), &cap, |b, &cap| {
			b.iter_batched(
				|| SpscRing::<u64>::with_capacity_pow2(cap),
				|ring| {
					let mut i = 0u64;
					while ring.push(black_box(i)).is_ok() { i = i.wrapping_add(1); }
				},
				BatchSize::SmallInput,
			);
		});
	}
	group.finish();
}

fn bench_spsc_consumer_only(c: &mut Criterion) {
	let mut group = c.benchmark_group("spsc_consumer_only");
	for &cap in &[64usize, 256, 4096] {
		group.bench_with_input(BenchmarkId::from_parameter(cap), &cap, |b, &cap| {
			b.iter_batched(
				|| {
					let ring = SpscRing::<u64>::with_capacity_pow2(cap);
					for i in 0..cap { let _ = ring.push(i as u64); }
					ring
				},
				|ring| { while let Some(_v) = ring.pop() { black_box(()); } },
				BatchSize::SmallInput,
			);
		});
	}
	group.finish();
}

fn bench_spsc_steady_state(c: &mut Criterion) {
	let mut group = c.benchmark_group("spsc_steady_state");
	for &cap in &[64usize, 256, 4096] {
		group.bench_with_input(BenchmarkId::from_parameter(cap), &cap, |b, &cap| {
            b.iter_batched(
                || Arc::new(SpscRing::<u64>::with_capacity_pow2(cap)),
                |ring| {
                    // Batched 32
                    let (mut p, mut c) = SpscRing::split_batched_owned::<32>(ring);
                    let prod = std::thread::spawn(move || {
                        let mut i = 0u64;
                        for _ in 0..(cap * 100) {
                            // busy wait until accepted
                            loop { match p.push(i) { Ok(_) => break, Err(x) => { i = x; std::hint::spin_loop(); } } }
                            i = i.wrapping_add(1);
                        }
                        p.flush();
                    });
                    let cons = std::thread::spawn(move || {
                        let mut cnt = 0usize;
                        while cnt < cap * 100 {
                            if c.pop().is_some() { cnt += 1; } else { std::hint::spin_loop(); }
                        }
                        c.flush();
                    });
                    let _ = prod.join();
                    let _ = cons.join();
                },
                BatchSize::SmallInput,
            );
		});
	}
	group.finish();
}

criterion_group!(benches, bench_spsc_producer_only, bench_spsc_consumer_only, bench_spsc_steady_state);
criterion_main!(benches);
