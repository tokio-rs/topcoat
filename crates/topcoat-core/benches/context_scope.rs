//! Benchmarks for scoped request context and scope-aware memoization.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use topcoat::context::{Cx, CxTestBuilder, memoize, request_context, try_request_context};

const SIBLING_MEMOIZE_CALLS: u64 = 6;

struct CurrentUser(u64);
struct Tenant(u64);
struct Locale(u64);
struct FeatureFlags(u64);
struct RequestMetadata(u64);
struct MissingValue;
struct ScopeValue(u64);

fn root_context() -> Cx {
    CxTestBuilder::new()
        .request_context(CurrentUser(11))
        .request_context(Tenant(13))
        .request_context(Locale(17))
        .request_context(FeatureFlags(19))
        .request_context(RequestMetadata(23))
        .build()
}

#[memoize]
fn tenant_value(cx: &Cx, key: u64) -> u64 {
    request_context::<Tenant>(cx).0.wrapping_add(key)
}

#[memoize]
fn optional_value(cx: &Cx, key: u64) -> u64 {
    try_request_context::<MissingValue>(cx).map_or(key, |_| key.wrapping_add(1))
}

#[memoize]
fn current_user(cx: &Cx, key: u64) -> u64 {
    request_context::<CurrentUser>(cx).0.wrapping_add(key)
}

#[memoize]
fn page_model(cx: &Cx, key: u64) -> u64 {
    current_user(cx, key).wrapping_mul(2)
}

fn nested_scopes(cx: &Cx, remaining: u64) -> u64 {
    if remaining == 0 {
        return request_context::<Tenant>(cx).0;
    }

    let scope = cx.with(Tenant(remaining));
    nested_scopes(&scope, remaining - 1)
}

fn bench_scope_creation(c: &mut Criterion) {
    let cx = root_context();
    let mut group = c.benchmark_group("cx_scope_create_drop");

    group.bench_function("new_binding", |b| {
        b.iter(|| {
            let scope = black_box(&cx).with(ScopeValue(black_box(29)));
            black_box(&scope);
        });
    });
    group.bench_function("shadow_binding", |b| {
        b.iter(|| {
            let scope = black_box(&cx).with(Tenant(black_box(29)));
            black_box(&scope);
        });
    });
    group.bench_function("four_bindings", |b| {
        b.iter(|| {
            let scope = black_box(&cx).with_values((
                Tenant(black_box(29)),
                Locale(black_box(31)),
                FeatureFlags(black_box(37)),
                RequestMetadata(black_box(41)),
            ));
            black_box(&scope);
        });
    });
    group.finish();
}

fn bench_scope_reads(c: &mut Criterion) {
    let cx = root_context();
    let owned = cx.with(ScopeValue(29));
    let inherited = cx.with(MissingValue);
    let shadowed = cx.with(Tenant(29));
    let level_one = cx.with(CurrentUser(31));
    let level_two = level_one.with(Locale(37));
    let level_three = level_two.with(FeatureFlags(41));
    let level_four = level_three.with(RequestMetadata(43));
    let mut group = c.benchmark_group("cx_scope_read");

    group.bench_function("owned_binding", |b| {
        b.iter(|| black_box(request_context::<ScopeValue>(black_box(&owned)).0));
    });
    group.bench_function("inherited_root_binding", |b| {
        b.iter(|| black_box(request_context::<CurrentUser>(black_box(&inherited)).0));
    });
    group.bench_function("shadowed_binding", |b| {
        b.iter(|| black_box(request_context::<Tenant>(black_box(&shadowed)).0));
    });
    group.bench_function("inherited_through_four_scopes", |b| {
        b.iter(|| black_box(request_context::<Tenant>(black_box(&level_four)).0));
    });

    black_box(request_context::<Locale>(&level_four).0);
    black_box(request_context::<FeatureFlags>(&level_four).0);
    black_box(request_context::<RequestMetadata>(&level_four).0);
    group.finish();
}

fn bench_nested_scopes(c: &mut Criterion) {
    let cx = root_context();
    let mut group = c.benchmark_group("cx_scope_nested_create_read_drop");

    for depth in [1, 4, 8] {
        group.bench_with_input(BenchmarkId::from_parameter(depth), &depth, |b, &depth| {
            b.iter(|| black_box(nested_scopes(black_box(&cx), depth)));
        });
    }
    group.finish();
}

fn bench_memoize_scope_hits(c: &mut Criterion) {
    let cx = root_context();
    let inherited = cx.with(MissingValue);
    let shadowed = cx.with(Tenant(29));
    let missing = cx.with(Locale(31));
    let key = black_box(43);

    black_box(tenant_value(&cx, key));
    black_box(tenant_value(&shadowed, key));
    black_box(optional_value(&cx, key));

    let mut group = c.benchmark_group("memoize_sync_hit_scope");
    group.bench_function("inherited_root_dependency", |b| {
        b.iter(|| black_box(tenant_value(black_box(&inherited), black_box(key))));
    });
    group.bench_function("shadowed_dependency", |b| {
        b.iter(|| black_box(tenant_value(black_box(&shadowed), black_box(key))));
    });
    group.bench_function("inherited_missing_dependency", |b| {
        b.iter(|| black_box(optional_value(black_box(&missing), black_box(key))));
    });
    group.finish();
}

fn bench_memoize_scope_variants(c: &mut Criterion) {
    let key = black_box(47);
    let mut group = c.benchmark_group("memoize_sync_scope_variants");

    group.bench_function("new_shadow_variant", |b| {
        b.iter_batched(
            root_context,
            |cx| {
                let scope = cx.with(Tenant(black_box(53)));
                black_box(*tenant_value(&scope, black_box(key)))
            },
            BatchSize::SmallInput,
        );
    });

    group.throughput(Throughput::Elements(SIBLING_MEMOIZE_CALLS));
    group.bench_function("root_and_two_siblings", |b| {
        b.iter_batched(
            root_context,
            |cx| {
                black_box(tenant_value(&cx, black_box(key)));
                black_box(tenant_value(&cx, black_box(key)));

                let first = cx.with(Tenant(59));
                black_box(tenant_value(&first, black_box(key)));
                black_box(tenant_value(&first, black_box(key)));
                drop(first);

                let second = cx.with(Tenant(61));
                black_box(tenant_value(&second, black_box(key)));
                black_box(tenant_value(&second, black_box(key)));
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_nested_memoize(c: &mut Criterion) {
    let cx = root_context();
    let inherited = cx.with(MissingValue);
    let shadowed = cx.with(CurrentUser(67));
    let key = black_box(71);

    black_box(page_model(&cx, key));
    black_box(page_model(&shadowed, key));

    let mut group = c.benchmark_group("memoize_nested_hit_scope");
    group.bench_function("inherited_dependency", |b| {
        b.iter(|| black_box(page_model(black_box(&inherited), black_box(key))));
    });
    group.bench_function("shadowed_dependency", |b| {
        b.iter(|| black_box(page_model(black_box(&shadowed), black_box(key))));
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_scope_creation,
    bench_scope_reads,
    bench_nested_scopes,
    bench_memoize_scope_hits,
    bench_memoize_scope_variants,
    bench_nested_memoize,
);
criterion_main!(benches);
