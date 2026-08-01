use std::sync::{
    Arc,
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

#[tokio::test]
async fn memoized_option_return_is_borrowed_ergonomically() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
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

#[test]
fn generic_monomorphizations_have_independent_slots() {
    #[memoize]
    fn default_value<T>(cx: &Cx, key: u8) -> T
    where
        T: Default + Send + Sync + 'static,
    {
        let _ = (cx, key);
        T::default()
    }

    let cx = Cx::default();
    let number: &u16 = default_value(&cx, 0);
    let text: &String = default_value(&cx, 0);

    assert_eq!(*number, 0);
    assert!(text.is_empty());
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
fn a_missing_context_read_is_a_dependency() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn optional_version(cx: &Cx) -> Option<u8> {
        CALLS.fetch_add(1, Ordering::SeqCst);
        try_request_context::<Version>(cx).map(|version| version.0)
    }

    let cx = Cx::default();
    assert_eq!(optional_version(&cx), None);

    cx.insert(Version(2));
    assert_eq!(optional_version(&cx), Some(&2));
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
}

#[test]
fn replacing_a_binding_invalidates_only_callers_that_inherit_it() {
    let cx = Cx::default();
    cx.insert(Version(1));
    let shadowed = cx.with(Version(9));

    let first_root = selected_version(&cx);
    let first_shadow = selected_version(&shadowed);
    cx.insert(Version(2));
    let second_root = selected_version(&cx);
    let second_shadow = selected_version(&shadowed);

    assert_eq!((*first_root, *first_shadow, *second_root), (1, 9, 2));
    assert!(!std::ptr::eq(first_root, second_root));
    assert!(std::ptr::eq(first_shadow, second_shadow));
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
fn bindings_created_inside_a_memoized_body_are_not_outer_dependencies() {
    let cx = Cx::default();
    let first = scoped_heading(&cx);
    let second = scoped_heading(&cx);

    assert_eq!(*first, 2);
    assert!(std::ptr::eq(first, second));
}

#[test]
fn inserting_from_a_memoized_body_panics_without_poisoning_the_key() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn invalid(cx: &Cx) -> usize {
        let call = CALLS.fetch_add(1, Ordering::SeqCst);
        if call == 0 {
            cx.insert(Version(1));
        }
        call
    }

    let cx = Cx::default();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| invalid(&cx)));
    assert!(panic.is_err());
    assert_eq!(*invalid(&cx), 1);
}

#[tokio::test]
async fn concurrent_calls_with_one_binding_share_computation() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    async fn selected_version(cx: &Cx) -> u8 {
        CALLS.fetch_add(1, Ordering::SeqCst);
        tokio::task::yield_now().await;
        request_context::<Version>(cx).0
    }

    let cx = Cx::default();
    let versioned = cx.with(Version(1));
    let (first, second) = tokio::join!(selected_version(&versioned), selected_version(&versioned));

    assert_eq!((*first, *second), (1, 1));
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn different_context_variants_for_one_key_run_serially() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);
    static ACTIVE: AtomicUsize = AtomicUsize::new(0);
    static MAX_ACTIVE: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    async fn selected_version(cx: &Cx) -> u8 {
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
    let (first, second) = tokio::join!(selected_version(&stable), selected_version(&next));

    assert_eq!((*first, *second), (1, 2));
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
    assert_eq!(MAX_ACTIVE.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn cancelled_computation_wakes_a_waiter_without_storing_a_value() {
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

struct Gates {
    started: tokio::sync::Notify,
    proceed: tokio::sync::Notify,
}

#[tokio::test]
async fn in_flight_calls_and_waiters_keep_their_captured_revision() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    async fn selected_version(cx: &Cx) -> u8 {
        let call = CALLS.fetch_add(1, Ordering::SeqCst);
        let version = request_context::<Version>(cx).0;
        if call == 0 {
            let gates = app_context::<Arc<Gates>>(cx);
            gates.started.notify_one();
            gates.proceed.notified().await;
        }
        version
    }

    let gates = Arc::new(Gates {
        started: tokio::sync::Notify::new(),
        proceed: tokio::sync::Notify::new(),
    });
    let cx = CxTestBuilder::new()
        .app_context(gates.clone())
        .request_context(Version(1))
        .build();

    let update = async {
        gates.started.notified().await;
        cx.insert(Version(2));
        gates.proceed.notify_one();
    };
    let (first, waiter, ()) = tokio::join!(selected_version(&cx), selected_version(&cx), update);

    assert_eq!((*first, *waiter), (1, 1));
    assert_eq!(*selected_version(&cx).await, 2);
    assert_eq!(CALLS.load(Ordering::SeqCst), 2);
}
