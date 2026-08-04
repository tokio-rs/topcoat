//! Benchmarks for root-context memoization.
//!
//! Hits use one long-lived request. Misses use batched setup to create a fresh
//! request outside the measurement, preventing unbounded cache growth and
//! keeping request construction out of the reported time.

use std::{
    future::Future,
    hint::black_box,
    pin::pin,
    task::{Context, Poll, Waker},
};

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use topcoat::context::{Cx, CxTestBuilder, memoize, request_context};

struct CurrentUser(u64);
struct Locale(u64);
struct FeatureFlags(u64);
struct RequestMetadata(u64);

fn root_context() -> Cx {
    CxTestBuilder::new()
        .request_context(CurrentUser(11))
        .request_context(Locale(13))
        .request_context(FeatureFlags(17))
        .request_context(RequestMetadata(19))
        .build()
}

#[memoize]
fn sync_zero_reads(cx: &Cx, key: u64) -> u64 {
    let _ = cx;
    key.wrapping_mul(31)
}

#[memoize]
fn sync_one_read(cx: &Cx, key: u64) -> u64 {
    request_context::<CurrentUser>(cx).0.wrapping_add(key)
}

#[memoize]
fn sync_four_reads(cx: &Cx, key: u64) -> u64 {
    request_context::<CurrentUser>(cx)
        .0
        .wrapping_add(request_context::<Locale>(cx).0)
        .wrapping_add(request_context::<FeatureFlags>(cx).0)
        .wrapping_add(request_context::<RequestMetadata>(cx).0)
        .wrapping_add(key)
}

#[memoize]
fn sync_borrowed_key(cx: &Cx, key: &str) -> usize {
    key.len()
        .wrapping_add(usize::try_from(request_context::<CurrentUser>(cx).0).unwrap_or_default())
}

#[memoize]
async fn async_zero_reads(cx: &Cx, key: u64) -> u64 {
    let _ = cx;
    key.wrapping_mul(37)
}

#[memoize]
async fn async_one_read(cx: &Cx, key: u64) -> u64 {
    request_context::<CurrentUser>(cx).0.wrapping_add(key)
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let mut task = Context::from_waker(Waker::noop());

    loop {
        if let Poll::Ready(output) = future.as_mut().poll(&mut task) {
            return output;
        }
        std::thread::yield_now();
    }
}

fn bench_sync_hits(c: &mut Criterion) {
    let cx = root_context();
    let key = black_box(23);
    let borrowed_key = black_box("current-user");

    black_box(sync_zero_reads(&cx, key));
    black_box(sync_one_read(&cx, key));
    black_box(sync_four_reads(&cx, key));
    black_box(sync_borrowed_key(&cx, borrowed_key));

    let mut group = c.benchmark_group("memoize_sync_hit_root");
    group.bench_function("zero_context_reads", |b| {
        b.iter(|| black_box(sync_zero_reads(black_box(&cx), black_box(key))));
    });
    group.bench_function("one_context_read", |b| {
        b.iter(|| black_box(sync_one_read(black_box(&cx), black_box(key))));
    });
    group.bench_function("four_context_reads", |b| {
        b.iter(|| black_box(sync_four_reads(black_box(&cx), black_box(key))));
    });
    group.bench_function("borrowed_string_key", |b| {
        b.iter(|| black_box(sync_borrowed_key(black_box(&cx), black_box(borrowed_key))));
    });
    group.finish();
}

fn bench_sync_misses(c: &mut Criterion) {
    let key = black_box(29);
    let borrowed_key = black_box("current-user");
    let mut group = c.benchmark_group("memoize_sync_miss_root");

    group.bench_function("zero_context_reads", |b| {
        b.iter_batched(
            root_context,
            |cx| black_box(*sync_zero_reads(&cx, black_box(key))),
            BatchSize::SmallInput,
        );
    });
    group.bench_function("one_context_read", |b| {
        b.iter_batched(
            root_context,
            |cx| black_box(*sync_one_read(&cx, black_box(key))),
            BatchSize::SmallInput,
        );
    });
    group.bench_function("four_context_reads", |b| {
        b.iter_batched(
            root_context,
            |cx| black_box(*sync_four_reads(&cx, black_box(key))),
            BatchSize::SmallInput,
        );
    });
    group.bench_function("borrowed_string_key", |b| {
        b.iter_batched(
            root_context,
            |cx| black_box(*sync_borrowed_key(&cx, black_box(borrowed_key))),
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_sync_fanout(c: &mut Criterion) {
    const UNIQUE_KEYS: u64 = 8;
    const CALLS: u64 = UNIQUE_KEYS * 2;

    let mut group = c.benchmark_group("memoize_sync_fanout_root");
    group.throughput(Throughput::Elements(CALLS));
    group.bench_function("eight_keys_called_twice", |b| {
        b.iter_batched(
            root_context,
            |cx| {
                for key in 0..UNIQUE_KEYS {
                    black_box(sync_one_read(&cx, black_box(key)));
                    black_box(sync_one_read(&cx, black_box(key)));
                }
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_async_hits(c: &mut Criterion) {
    let cx = root_context();
    let key = black_box(31);

    black_box(block_on(async_zero_reads(&cx, key)));
    black_box(block_on(async_one_read(&cx, key)));

    let mut group = c.benchmark_group("memoize_async_hit_root");
    group.bench_function("zero_context_reads", |b| {
        b.iter(|| black_box(block_on(async_zero_reads(black_box(&cx), black_box(key)))));
    });
    group.bench_function("one_context_read", |b| {
        b.iter(|| black_box(block_on(async_one_read(black_box(&cx), black_box(key)))));
    });
    group.finish();
}

fn bench_async_misses(c: &mut Criterion) {
    let key = black_box(37);
    let mut group = c.benchmark_group("memoize_async_miss_root");

    group.bench_function("zero_context_reads", |b| {
        b.iter_batched(
            root_context,
            |cx| black_box(*block_on(async_zero_reads(&cx, black_box(key)))),
            BatchSize::SmallInput,
        );
    });
    group.bench_function("one_context_read", |b| {
        b.iter_batched(
            root_context,
            |cx| black_box(*block_on(async_one_read(&cx, black_box(key)))),
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_sync_hits,
    bench_sync_misses,
    bench_sync_fanout,
    bench_async_hits,
    bench_async_misses,
);
criterion_main!(benches);
