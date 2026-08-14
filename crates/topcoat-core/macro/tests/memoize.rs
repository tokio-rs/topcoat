use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
};

use topcoat::context::{
    Cx, CxTestBuilder, app_context, memoize, request_context, try_request_context,
};

#[tokio::test]
async fn sync_memoized_function_runs_body_once_per_key_per_request() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn add(cx: &Cx, x: i32, y: i32) -> i32 {
        let _ = cx;
        CALLS.fetch_add(1, Ordering::SeqCst);
        x + y
    }

    let cx = Cx::default();

    let a = add(&cx, 1, 2);
    let b = add(&cx, 1, 2);
    let c = add(&cx, 1, 3);

    assert_eq!(*a, 3);
    assert_eq!(*b, 3);
    assert_eq!(*c, 4);
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn sync_memoized_function_cache_does_not_cross_requests() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn add(cx: &Cx, x: i32, y: i32) -> i32 {
        let _ = cx;
        CALLS.fetch_add(1, Ordering::SeqCst);
        x + y
    }

    let cx1 = Cx::default();
    let cx2 = Cx::default();

    add(&cx1, 7, 7);
    add(&cx2, 7, 7);

    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn async_memoized_function_runs_body_once_per_key_per_request() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    async fn fetch(cx: &Cx, slug: &str) -> String {
        let _ = cx;
        CALLS.fetch_add(1, Ordering::SeqCst);
        format!("post:{slug}")
    }

    let cx = Cx::default();

    let a = fetch(&cx, "hello").await;
    let b = fetch(&cx, "hello").await;
    let c = fetch(&cx, "world").await;

    assert_eq!(a, "post:hello");
    assert_eq!(b, "post:hello");
    assert_eq!(c, "post:world");
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn async_memoized_function_returns_stable_reference() {
    #[memoize]
    async fn fetch(cx: &Cx, slug: &str) -> String {
        let _ = cx;
        format!("post:{slug}")
    }

    let cx = Cx::default();

    let first: &String = fetch(&cx, "same").await;
    let second: &String = fetch(&cx, "same").await;

    assert!(std::ptr::eq(first.as_ptr(), second.as_ptr()));
}

#[test]
fn memoized_futures_are_send() {
    fn assert_send<T: Send>(_: T) {}

    #[memoize]
    async fn value(cx: &Cx) -> usize {
        let _ = cx;
        tokio::task::yield_now().await;
        1
    }

    let cx = Cx::default();
    assert_send(value(&cx));
}

#[tokio::test]
async fn memoized_option_return_is_borrowed_ergonomically_with_as_ref() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize(as_ref)]
    fn maybe(cx: &Cx, is_some: bool) -> Option<i32> {
        let _ = cx;
        CALLS.fetch_add(1, Ordering::SeqCst);
        if is_some { Some(42) } else { None }
    }

    let cx = Cx::default();

    let some_value: Option<&i32> = maybe(&cx, true);
    let none_value: Option<&i32> = maybe(&cx, false);

    assert_eq!(some_value, Some(&42));
    assert_eq!(none_value, None);

    maybe(&cx, true);
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn memoized_result_return_is_borrowed_ergonomically_with_as_ref() {
    #[memoize(as_ref)]
    fn fallible(cx: &Cx, fail: bool) -> Result<i32, String> {
        let _ = cx;
        if fail { Err("nope".to_owned()) } else { Ok(42) }
    }

    let cx = Cx::default();

    let ok_value: Result<&i32, &String> = fallible(&cx, false);
    let err_value: Result<&i32, &String> = fallible(&cx, true);

    assert_eq!(ok_value, Ok(&42));
    assert_eq!(err_value, Err(&"nope".to_owned()));
}

#[tokio::test]
async fn memoized_option_return_is_a_plain_reference_by_default() {
    #[memoize]
    fn maybe(cx: &Cx, is_some: bool) -> Option<i32> {
        let _ = cx;
        if is_some { Some(42) } else { None }
    }

    let cx = Cx::default();

    let value: &Option<i32> = maybe(&cx, true);

    assert_eq!(value, &Some(42));
}

#[tokio::test]
async fn separate_memoized_functions_have_independent_caches() {
    static A_CALLS: AtomicUsize = AtomicUsize::new(0);
    static B_CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn fn_a(cx: &Cx, x: i32) -> i32 {
        let _ = cx;
        A_CALLS.fetch_add(1, Ordering::SeqCst);
        x
    }

    #[memoize]
    fn fn_b(cx: &Cx, x: i32) -> i32 {
        let _ = cx;
        B_CALLS.fetch_add(1, Ordering::SeqCst);
        x * 10
    }

    let cx = Cx::default();

    assert_eq!(*fn_a(&cx, 1), 1);
    assert_eq!(*fn_b(&cx, 1), 10);
    assert_eq!(*fn_a(&cx, 1), 1);
    assert_eq!(*fn_b(&cx, 1), 10);

    assert_eq!(A_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(B_CALLS.load(Ordering::SeqCst), 1);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Version(u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HeadingLevel(u8);

#[memoize]
fn selected_version(cx: &Cx) -> u8 {
    request_context::<Version>(cx).0
}

#[memoize]
fn outer_version(cx: &Cx) -> u8 {
    *selected_version(cx)
}

#[memoize]
fn inner_heading(cx: &Cx) -> u8 {
    request_context::<HeadingLevel>(cx).0
}

#[memoize]
fn scoped_heading(cx: &Cx) -> u8 {
    let section = cx.with(HeadingLevel(2));
    *inner_heading(&section)
}

fn counted_cx() -> Cx {
    CxTestBuilder::new()
        .app_context(AtomicUsize::new(0))
        .build()
}

fn versioned_cx() -> Cx {
    CxTestBuilder::new()
        .app_context(AtomicUsize::new(0))
        .request_context(Version(1))
        .build()
}

fn calls(cx: &Cx) -> &AtomicUsize {
    app_context(cx)
}

#[memoize(as_ref)]
fn optional_version(cx: &Cx) -> Option<u8> {
    calls(cx).fetch_add(1, Ordering::SeqCst);
    try_request_context::<Version>(cx).map(|version| version.0)
}

#[memoize]
fn version_is_missing(cx: &Cx) -> bool {
    calls(cx).fetch_add(1, Ordering::SeqCst);
    try_request_context::<Version>(cx).is_none()
}

#[memoize]
fn counted_version(cx: &Cx) -> u8 {
    calls(cx).fetch_add(1, Ordering::SeqCst);
    request_context::<Version>(cx).0
}

#[test]
fn context_binding_identity_selects_and_retains_variants() {
    let cx = Cx::default();
    let first = cx.with(Version(1));
    let second = cx.with(Version(2));

    let first_value = selected_version(&first);
    let second_value = selected_version(&second);
    let first_again = selected_version(&first);

    assert_eq!((*first_value, *second_value), (1, 2));
    assert!(std::ptr::eq(first_value, first_again));
}

#[test]
fn equal_values_in_sibling_scopes_have_distinct_variants() {
    let cx = Cx::default();
    let first = cx.with(Version(1));
    let second = cx.with(Version(1));

    let first_value = selected_version(&first);
    let second_value = selected_version(&second);

    assert_eq!((*first_value, *second_value), (1, 1));
    assert!(!std::ptr::eq(first_value, second_value));
}

#[test]
fn unrelated_scoped_values_do_not_prevent_reuse() {
    let cx = Cx::default();
    let versioned = cx.with(Version(1));
    let section = versioned.with(HeadingLevel(2));

    let first = selected_version(&versioned);
    let second = selected_version(&section);

    assert_eq!((*first, *second), (1, 1));
    assert!(std::ptr::eq(first, second));
}

#[test]
fn shadowing_an_older_dependency_invalidates_with_the_youngest_unchanged() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn selected(cx: &Cx) -> (u8, u8) {
        CALLS.fetch_add(1, Ordering::SeqCst);
        (
            request_context::<Version>(cx).0,
            request_context::<HeadingLevel>(cx).0,
        )
    }

    let cx = CxTestBuilder::new().request_context(Version(1)).build();
    let section = cx.with(HeadingLevel(2));
    assert_eq!(*selected(&section), (1, 2));

    let shadowed = section.with(Version(9));
    assert_eq!(*selected(&shadowed), (9, 2));
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
}

#[test]
fn missing_reads_reuse_a_variant_across_preexisting_sibling_scopes() {
    let cx = counted_cx();
    let first = cx.with(HeadingLevel(1));
    let second = cx.with(HeadingLevel(2));

    let first_value = version_is_missing(&first);
    let second_value = version_is_missing(&second);

    assert!(*first_value);
    assert!(std::ptr::eq(first_value, second_value));
    assert_eq!(calls(&cx).load(Ordering::SeqCst), 1);
}

#[test]
fn preexisting_scope_keeps_implicit_none_after_first_scoped_value() {
    let cx = counted_cx();
    let before = cx.with(HeadingLevel(1));
    let versioned = cx.with(Version(2));

    assert_eq!(optional_version(&versioned), Some(&2));
    assert_eq!(optional_version(&before), None);
    assert_eq!(calls(&cx).load(Ordering::SeqCst), 2);
}

#[test]
fn a_missing_context_read_is_invalidated_by_root_insertion() {
    let mut cx = counted_cx();
    assert_eq!(optional_version(&cx), None);

    cx.insert(Version(2));

    assert_eq!(optional_version(&cx), Some(&2));
    assert_eq!(calls(&cx).load(Ordering::SeqCst), 2);
}

#[test]
fn unrelated_root_insertion_preserves_a_cached_missing_read() {
    let mut cx = counted_cx();
    assert_eq!(optional_version(&cx), None);

    cx.insert(HeadingLevel(2));

    assert_eq!(optional_version(&cx), None);
    assert_eq!(calls(&cx).load(Ordering::SeqCst), 1);
}

#[test]
fn missing_variant_is_reused_after_a_scoped_value_variant_is_dropped() {
    let cx = counted_cx();
    assert_eq!(optional_version(&cx), None);
    {
        let versioned = cx.with(Version(2));
        assert_eq!(optional_version(&versioned), Some(&2));
    }
    assert_eq!(optional_version(&cx), None);

    assert_eq!(calls(&cx).load(Ordering::SeqCst), 2);
}

#[test]
fn replacing_a_root_binding_invalidates_its_readers() {
    let mut cx = versioned_cx();
    assert_eq!(*counted_version(&cx), 1);
    assert_eq!(cx.insert(Version(1)), Some(Version(1)));
    assert_eq!(*counted_version(&cx), 1);

    assert_eq!(calls(&cx).load(Ordering::SeqCst), 2);
}

#[test]
fn successful_get_mut_invalidates_even_without_a_value_change() {
    let mut cx = versioned_cx();
    assert_eq!(*counted_version(&cx), 1);
    let _ = cx.get_mut::<Version>().unwrap();
    assert_eq!(*counted_version(&cx), 1);

    assert_eq!(calls(&cx).load(Ordering::SeqCst), 2);
}

#[test]
fn failed_get_mut_does_not_invalidate_a_missing_read() {
    let mut cx = counted_cx();
    assert_eq!(optional_version(&cx), None);
    assert_eq!(cx.get_mut::<Version>(), None);
    assert_eq!(optional_version(&cx), None);

    assert_eq!(calls(&cx).load(Ordering::SeqCst), 1);
}

#[test]
fn unrelated_root_mutation_does_not_invalidate_a_result() {
    let mut cx = versioned_cx();
    assert_eq!(*counted_version(&cx), 1);
    cx.insert(HeadingLevel(2));
    assert_eq!(*counted_version(&cx), 1);

    assert_eq!(calls(&cx).load(Ordering::SeqCst), 1);
}

#[test]
fn nested_memoized_calls_propagate_computed_dependencies() {
    let cx = Cx::default();
    let stable = cx.with(Version(1));
    let next = cx.with(Version(2));

    assert_eq!(*outer_version(&stable), 1);
    assert_eq!(*outer_version(&next), 2);
}

#[test]
fn nested_cache_hits_propagate_dependencies() {
    let cx = Cx::default();
    let stable = cx.with(Version(1));
    let next = cx.with(Version(2));

    assert_eq!(*selected_version(&stable), 1);
    assert_eq!(*outer_version(&stable), 1);
    assert_eq!(*outer_version(&next), 2);
}

#[test]
fn nested_missing_cache_hits_propagate_dependencies() {
    static OUTER_CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn inner(cx: &Cx) -> bool {
        try_request_context::<Version>(cx).is_some()
    }

    #[memoize]
    fn outer(cx: &Cx) -> bool {
        OUTER_CALLS.fetch_add(1, Ordering::SeqCst);
        *inner(cx)
    }

    let mut cx = Cx::default();
    assert!(!*inner(&cx));
    assert!(!*outer(&cx));

    cx.insert(Version(1));

    assert!(*outer(&cx));
    assert_eq!(OUTER_CALLS.load(Ordering::SeqCst), 2);
}

#[test]
fn nested_missing_hit_propagates_into_an_active_outer_call() {
    static OUTER_CALLS: AtomicUsize = AtomicUsize::new(0);

    struct Gates {
        outer_started: Barrier,
        inner_ready: Barrier,
    }

    #[memoize]
    fn inner(cx: &Cx) -> bool {
        try_request_context::<Version>(cx).is_some()
    }

    #[memoize]
    fn outer(cx: &Cx) -> bool {
        let call = OUTER_CALLS.fetch_add(1, Ordering::SeqCst);
        if call == 0 {
            let gates = app_context::<Gates>(cx);
            gates.outer_started.wait();
            gates.inner_ready.wait();
        }
        *inner(cx)
    }

    let mut cx = CxTestBuilder::new()
        .app_context(Gates {
            outer_started: Barrier::new(2),
            inner_ready: Barrier::new(2),
        })
        .build();

    std::thread::scope(|scope| {
        let worker = scope.spawn(|| *outer(&cx));
        let gates = app_context::<Gates>(&cx);
        gates.outer_started.wait();
        assert!(!*inner(&cx));
        gates.inner_ready.wait();
        assert!(!worker.join().unwrap());
    });

    cx.insert(Version(1));

    assert!(*outer(&cx));
    assert_eq!(OUTER_CALLS.load(Ordering::SeqCst), 2);
}

#[test]
fn bindings_created_inside_a_memoized_body_are_not_outer_dependencies() {
    let cx = Cx::default();
    let first = scoped_heading(&cx);
    let caller_heading = cx.with(HeadingLevel(9));
    let second = scoped_heading(&caller_heading);

    assert_eq!(*first, 2);
    assert!(std::ptr::eq(first, second));
}

#[test]
fn internal_bindings_are_excluded_while_inherited_missing_reads_propagate() {
    static OUTER_CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn inner(cx: &Cx) -> (u8, Option<u8>) {
        (
            request_context::<HeadingLevel>(cx).0,
            try_request_context::<Version>(cx).map(|version| version.0),
        )
    }

    #[memoize]
    fn outer(cx: &Cx) -> (u8, Option<u8>) {
        OUTER_CALLS.fetch_add(1, Ordering::SeqCst);
        let child = cx.with(HeadingLevel(2));
        *inner(&child)
    }

    let mut cx = Cx::default();
    assert_eq!(*outer(&cx), (2, None));

    cx.insert(Version(9));

    assert_eq!(*outer(&cx), (2, Some(9)));
    assert_eq!(OUTER_CALLS.load(Ordering::SeqCst), 2);
}

#[test]
fn reads_from_multiple_internal_scopes_are_not_inputs() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    fn versions(first: &Cx, second: &Cx) -> (u8, u8) {
        (
            request_context::<Version>(first).0,
            request_context::<Version>(second).0,
        )
    }

    #[memoize]
    fn selected(cx: &Cx) -> (u8, u8) {
        CALLS.fetch_add(1, Ordering::SeqCst);
        let first = cx.with(Version(1));
        let second = cx.with(Version(2));
        versions(&first, &second)
    }

    let cx = Cx::default();
    assert_eq!(*selected(&cx), (1, 2));

    let caller_version = cx.with(Version(9));
    assert_eq!(*selected(&caller_version), (1, 2));
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
}

#[test]
fn app_context_reads_are_not_dependencies() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn configured(cx: &Cx) -> u8 {
        CALLS.fetch_add(1, Ordering::SeqCst);
        app_context::<Version>(cx).0
    }

    let cx = CxTestBuilder::new().app_context(Version(7)).build();
    let first = cx.with(HeadingLevel(1));
    let second = cx.with(HeadingLevel(2));

    assert_eq!(*configured(&first), 7);
    assert_eq!(*configured(&second), 7);
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
}

#[test]
fn nested_reads_from_another_request_do_not_propagate_on_misses_or_hits() {
    static OUTER_CALLS: AtomicUsize = AtomicUsize::new(0);

    struct OtherRequest(Arc<Cx>);

    #[memoize]
    fn inner(cx: &Cx) -> u8 {
        request_context::<Version>(cx).0
    }

    #[memoize]
    fn outer(cx: &Cx) -> u8 {
        OUTER_CALLS.fetch_add(1, Ordering::SeqCst);
        *inner(&app_context::<OtherRequest>(cx).0)
    }

    for prewarm in [false, true] {
        OUTER_CALLS.store(0, Ordering::SeqCst);
        let other = Arc::new(CxTestBuilder::new().request_context(Version(1)).build());
        if prewarm {
            assert_eq!(*inner(&other), 1);
        }
        let cx = CxTestBuilder::new()
            .app_context(OtherRequest(other))
            .request_context(Version(9))
            .build();

        assert_eq!(*outer(&cx), 1);
        let shadowed = cx.with(Version(8));
        assert_eq!(*outer(&shadowed), 1);
        assert_eq!(OUTER_CALLS.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn sync_panicking_computation_can_be_retried() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn flaky(cx: &Cx) -> usize {
        let _ = cx;
        let call = CALLS.fetch_add(1, Ordering::SeqCst);
        assert_ne!(call, 0, "first call panics");
        call
    }

    let cx = Cx::default();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| flaky(&cx)));

    assert!(panic.is_err());
    assert_eq!(*flaky(&cx), 1);
}

#[test]
fn concurrent_sync_calls_with_one_binding_share_computation() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn slow(cx: &Cx) -> usize {
        let _ = cx;
        CALLS.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(10));
        7
    }

    let cx = Cx::default();
    std::thread::scope(|scope| {
        let first = scope.spawn(|| *slow(&cx));
        let second = scope.spawn(|| *slow(&cx));
        assert_eq!((first.join().unwrap(), second.join().unwrap()), (7, 7));
    });

    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
}

#[test]
fn different_sync_context_variants_for_one_key_run_serially() {
    static ACTIVE: AtomicUsize = AtomicUsize::new(0);
    static MAX_ACTIVE: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn selected(cx: &Cx) -> u8 {
        let active = ACTIVE.fetch_add(1, Ordering::SeqCst) + 1;
        MAX_ACTIVE.fetch_max(active, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(10));
        ACTIVE.fetch_sub(1, Ordering::SeqCst);
        request_context::<Version>(cx).0
    }

    let cx = Cx::default();
    let first_cx = cx.with(Version(1));
    let second_cx = cx.with(Version(2));
    std::thread::scope(|scope| {
        let first = scope.spawn(|| *selected(&first_cx));
        let second = scope.spawn(|| *selected(&second_cx));
        assert_eq!((first.join().unwrap(), second.join().unwrap()), (1, 2));
    });

    assert_eq!(MAX_ACTIVE.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn concurrent_calls_with_one_binding_share_computation() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    async fn selected(cx: &Cx) -> u8 {
        CALLS.fetch_add(1, Ordering::SeqCst);
        tokio::task::yield_now().await;
        request_context::<Version>(cx).0
    }

    let cx = Cx::default();
    let versioned = cx.with(Version(1));
    let (first, second) = tokio::join!(selected(&versioned), selected(&versioned));

    assert_eq!((*first, *second), (1, 1));
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn different_context_variants_for_one_key_run_serially() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    static ACTIVE: AtomicUsize = AtomicUsize::new(0);
    static MAX_ACTIVE: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    async fn selected(cx: &Cx) -> u8 {
        CALLS.fetch_add(1, Ordering::SeqCst);
        let active = ACTIVE.fetch_add(1, Ordering::SeqCst) + 1;
        MAX_ACTIVE.fetch_max(active, Ordering::SeqCst);
        tokio::task::yield_now().await;
        ACTIVE.fetch_sub(1, Ordering::SeqCst);
        request_context::<Version>(cx).0
    }

    let cx = Cx::default();
    let stable = cx.with(Version(1));
    let next = cx.with(Version(2));
    let (first, second) = tokio::join!(selected(&stable), selected(&next));

    assert_eq!((*first, *second), (1, 2));
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
    assert_eq!(MAX_ACTIVE.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn async_nested_calls_propagate_dependencies_across_polls() {
    static INNER_CALLS: AtomicUsize = AtomicUsize::new(0);
    static OUTER_CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    async fn inner(cx: &Cx, key: u8) -> u8 {
        INNER_CALLS.fetch_add(1, Ordering::SeqCst);
        tokio::task::yield_now().await;
        let version = request_context::<Version>(cx).0;
        tokio::task::yield_now().await;
        version + key
    }

    #[memoize]
    async fn outer(cx: &Cx, key: u8) -> u8 {
        OUTER_CALLS.fetch_add(1, Ordering::SeqCst);
        tokio::task::yield_now().await;
        *inner(cx, key).await
    }

    let cx = Cx::default();
    let stable = cx.with(Version(1));
    let next = cx.with(Version(2));
    let (first, second) = tokio::join!(outer(&stable, 10), outer(&next, 20));

    assert_eq!((*first, *second), (11, 22));
    assert!(std::ptr::eq(first, outer(&stable, 10).await));
    assert!(std::ptr::eq(second, outer(&next, 20).await));
    assert_eq!(INNER_CALLS.load(Ordering::SeqCst), 2);
    assert_eq!(OUTER_CALLS.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn different_argument_keys_run_concurrently() {
    static ACTIVE: AtomicUsize = AtomicUsize::new(0);
    static MAX_ACTIVE: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    async fn keyed(cx: &Cx, key: u8) -> u8 {
        let active = ACTIVE.fetch_add(1, Ordering::SeqCst) + 1;
        MAX_ACTIVE.fetch_max(active, Ordering::SeqCst);
        tokio::task::yield_now().await;
        let version = request_context::<Version>(cx).0;
        tokio::task::yield_now().await;
        ACTIVE.fetch_sub(1, Ordering::SeqCst);
        version + key
    }

    let cx = Cx::default();
    let stable = cx.with(Version(1));
    let next = cx.with(Version(2));
    let (first, second) = tokio::join!(keyed(&stable, 10), keyed(&next, 20));

    assert_eq!((*first, *second), (11, 22));
    assert!(std::ptr::eq(first, keyed(&stable, 10).await));
    assert!(std::ptr::eq(second, keyed(&next, 20).await));
    assert_eq!(MAX_ACTIVE.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn cancelled_computation_stores_nothing_and_can_be_retried() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    async fn cancellable(cx: &Cx) -> usize {
        let _ = cx;
        let call = CALLS.fetch_add(1, Ordering::SeqCst);
        if call == 0 {
            std::future::pending().await
        } else {
            call
        }
    }

    let cx = Cx::default();
    {
        let first = cancellable(&cx);
        tokio::pin!(first);
        tokio::select! {
            biased;
            _ = &mut first => panic!("the first computation should stay pending"),
            () = tokio::task::yield_now() => {}
        }
    }

    assert_eq!(*cancellable(&cx).await, 1);
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn async_panicking_computation_can_be_retried() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    async fn flaky(cx: &Cx) -> usize {
        let _ = cx;
        let call = CALLS.fetch_add(1, Ordering::SeqCst);
        assert_ne!(call, 0, "first call panics");
        call
    }

    let cx = Arc::new(Cx::default());
    let worker_cx = cx.clone();
    let worker = tokio::spawn(async move { *flaky(&worker_cx).await });

    assert!(worker.await.is_err());
    assert_eq!(*flaky(&cx).await, 1);
}

#[tokio::test]
async fn sync_direct_and_indirect_recursion_with_different_keys() {
    #[memoize]
    fn direct(cx: &Cx, depth: usize) -> usize {
        if depth == 0 {
            0
        } else {
            1 + *direct(cx, depth - 1)
        }
    }

    #[memoize]
    fn a(cx: &Cx, depth: usize) -> usize {
        if depth == 0 { 0 } else { 1 + *b(cx, depth - 1) }
    }

    #[memoize]
    fn b(cx: &Cx, depth: usize) -> usize {
        if depth == 0 { 0 } else { 1 + *c(cx, depth - 1) }
    }

    #[memoize]
    fn c(cx: &Cx, depth: usize) -> usize {
        if depth == 0 { 0 } else { 1 + *a(cx, depth - 1) }
    }

    let cx = Cx::default();

    assert_eq!(*direct(&cx, 5), 5);
    assert_eq!(*a(&cx, 5), 5);
}

#[tokio::test]
async fn async_direct_and_indirect_recursion_with_different_keys() {
    #[memoize]
    async fn direct(cx: &Cx, depth: usize) -> usize {
        if depth == 0 {
            0
        } else {
            1 + *Box::pin(direct(cx, depth - 1)).await
        }
    }

    #[memoize]
    async fn a(cx: &Cx, depth: usize) -> usize {
        if depth == 0 {
            0
        } else {
            1 + *b(cx, depth - 1).await
        }
    }

    #[memoize]
    async fn b(cx: &Cx, depth: usize) -> usize {
        if depth == 0 {
            0
        } else {
            1 + *c(cx, depth - 1).await
        }
    }

    #[memoize]
    async fn c(cx: &Cx, depth: usize) -> usize {
        if depth == 0 {
            0
        } else {
            1 + *Box::pin(a(cx, depth - 1)).await
        }
    }

    let cx = Cx::default();

    assert_eq!(*direct(&cx, 5).await, 5);
    assert_eq!(*a(&cx, 5).await, 5);
}

#[tokio::test]
#[should_panic(expected = "recursive `#[memoize]` initialization")]
async fn sync_direct_recursion_with_same_key_panics() {
    #[memoize]
    fn recursive(cx: &Cx) -> usize {
        *recursive(cx)
    }

    recursive(&Cx::default());
}

#[tokio::test]
#[should_panic(expected = "recursive `#[memoize]` initialization")]
async fn sync_indirect_recursion_with_same_key_panics() {
    #[memoize]
    fn a(cx: &Cx) -> usize {
        *b(cx)
    }

    #[memoize]
    fn b(cx: &Cx) -> usize {
        *c(cx)
    }

    #[memoize]
    fn c(cx: &Cx) -> usize {
        *a(cx)
    }

    a(&Cx::default());
}

#[tokio::test]
#[should_panic(expected = "recursive `#[memoize]` initialization")]
async fn async_direct_recursion_with_same_key_panics() {
    #[memoize]
    async fn recursive(cx: &Cx) -> usize {
        *Box::pin(recursive(cx)).await
    }

    recursive(&Cx::default()).await;
}

#[tokio::test]
#[should_panic(expected = "recursive `#[memoize]` initialization")]
async fn async_indirect_recursion_with_same_key_panics() {
    #[memoize]
    async fn a(cx: &Cx) -> usize {
        *b(cx).await
    }

    #[memoize]
    async fn b(cx: &Cx) -> usize {
        *c(cx).await
    }

    #[memoize]
    async fn c(cx: &Cx) -> usize {
        *Box::pin(a(cx)).await
    }

    a(&Cx::default()).await;
}
