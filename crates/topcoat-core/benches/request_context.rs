//! Benchmarks for request construction and root context lookups.
//!
//! These cases intentionally use no child scopes. They represent request
//! setup and the context reads performed by ordinary handlers and helpers.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use topcoat::context::{Cx, CxTestBuilder, request_context, try_request_context};

struct CurrentUser(u64);
struct Locale(&'static str);
struct FeatureFlags(u64);
struct RequestMetadata(u64);
struct MissingValue;

fn root_context() -> Cx {
    CxTestBuilder::new()
        .request_context(CurrentUser(42))
        .request_context(Locale("en-US"))
        .request_context(FeatureFlags(0b1010))
        .request_context(RequestMetadata(7))
        .build()
}

fn bench_request_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("cx_request_build");

    group.bench_function("empty", |b| {
        b.iter(|| black_box(Cx::default()));
    });
    group.bench_function("one_root_value", |b| {
        b.iter(|| {
            black_box(
                CxTestBuilder::new()
                    .request_context(CurrentUser(black_box(42)))
                    .build(),
            )
        });
    });
    group.bench_function("four_root_values", |b| {
        b.iter(|| black_box(root_context()));
    });

    group.finish();
}

fn bench_root_reads(c: &mut Criterion) {
    let cx = root_context();
    let mut group = c.benchmark_group("cx_request_read_root");

    group.bench_function("required_present", |b| {
        b.iter(|| black_box(request_context::<CurrentUser>(black_box(&cx)).0));
    });
    group.bench_function("optional_present", |b| {
        b.iter(|| {
            black_box(
                try_request_context::<Locale>(black_box(&cx))
                    .map(|locale| locale.0)
                    .is_some(),
            )
        });
    });
    group.bench_function("optional_missing", |b| {
        b.iter(|| black_box(try_request_context::<MissingValue>(black_box(&cx)).is_none()));
    });

    black_box(request_context::<FeatureFlags>(&cx).0);
    black_box(request_context::<RequestMetadata>(&cx).0);
    group.finish();
}

criterion_group!(benches, bench_request_build, bench_root_reads);
criterion_main!(benches);
