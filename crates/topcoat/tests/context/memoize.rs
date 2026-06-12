use std::sync::atomic::{AtomicUsize, Ordering};

use topcoat::context::{Cx, memoize};

#[tokio::test]
async fn sync_memoized_function_runs_body_once_per_key_per_request() {
    static CALLS: AtomicUsize = AtomicUsize::new(0);

    #[memoize]
    fn add(cx: &Cx, x: i32, y: i32) -> i32 {
        let _ = cx;
        CALLS.fetch_add(1, Ordering::SeqCst);
        x + y
    }

    let cx = Cx::empty();

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

    let cx1 = Cx::empty();
    let cx2 = Cx::empty();

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

    let cx = Cx::empty();

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

    let cx = Cx::empty();

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

    let cx = Cx::empty();

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

    let cx = Cx::empty();

    assert_eq!(*fn_a(&cx, 1), 1);
    assert_eq!(*fn_b(&cx, 1), 10);
    assert_eq!(*fn_a(&cx, 1), 1);
    assert_eq!(*fn_b(&cx, 1), 10);

    assert_eq!(A_CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(B_CALLS.load(Ordering::SeqCst), 1);
}
