use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
};

use topcoat_core::context::{
    Cx, CxScope, CxTestBuilder, app_context, memoize_cache, request_context, try_request_context,
};

#[derive(Debug, PartialEq)]
struct Database(&'static str);

#[derive(Debug, PartialEq)]
struct Config(u32);

fn counted_cx() -> Cx {
    CxTestBuilder::new()
        .app_context(AtomicUsize::new(0))
        .build()
}

fn calls(cx: &Cx) -> &AtomicUsize {
    app_context(cx)
}

fn optional_database(cx: &Cx) -> Option<&'static str> {
    memoize_cache(cx)
        .eq_cache()
        .memoize(cx, (), (), |cx, ()| {
            calls(cx).fetch_add(1, Ordering::SeqCst);
            try_request_context::<Database>(cx).map(|database| database.0)
        })
        .as_ref()
        .copied()
}

fn optional_config(cx: &Cx) -> Option<u32> {
    memoize_cache(cx)
        .eq_cache()
        .memoize(cx, (), (), |cx, ()| {
            calls(cx).fetch_add(1, Ordering::SeqCst);
            try_request_context::<Config>(cx).map(|config| config.0)
        })
        .as_ref()
        .copied()
}

struct ConcurrentReads {
    ready: Barrier,
    calls: AtomicUsize,
}

fn first_database_is_missing(cx: &Cx) -> &bool {
    memoize_cache(cx).eq_cache().memoize(cx, (), (), |cx, ()| {
        let state = app_context::<ConcurrentReads>(cx);
        let call = state.calls.fetch_add(1, Ordering::SeqCst);
        if call < 2 {
            state.ready.wait();
        }
        try_request_context::<Database>(cx).is_none()
    })
}

fn second_database_is_missing(cx: &Cx) -> &bool {
    memoize_cache(cx).eq_cache().memoize(cx, (), (), |cx, ()| {
        let state = app_context::<ConcurrentReads>(cx);
        let call = state.calls.fetch_add(1, Ordering::SeqCst);
        if call < 2 {
            state.ready.wait();
        }
        try_request_context::<Database>(cx).is_none()
    })
}

struct DropCounter(Arc<AtomicUsize>);

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn concurrent_missing_reads_are_invalidated_by_root_insertion() {
    let mut cx = CxTestBuilder::new()
        .app_context(ConcurrentReads {
            ready: Barrier::new(2),
            calls: AtomicUsize::new(0),
        })
        .build();

    std::thread::scope(|scope| {
        let first = scope.spawn(|| *first_database_is_missing(&cx));
        let second = scope.spawn(|| *second_database_is_missing(&cx));
        assert!(first.join().unwrap());
        assert!(second.join().unwrap());
    });

    cx.insert(Database("primary"));

    assert!(!first_database_is_missing(&cx));
    assert!(!second_database_is_missing(&cx));
    assert_eq!(
        app_context::<ConcurrentReads>(&cx)
            .calls
            .load(Ordering::SeqCst),
        4
    );
}

#[test]
fn a_scope_created_before_a_missing_read_reuses_its_cached_variant() {
    let cx = counted_cx();
    let before = cx.with(Config(1));

    assert_eq!(optional_database(&cx), None);
    assert_eq!(optional_database(&before), None);
    assert_eq!(calls(&cx).load(Ordering::SeqCst), 1);
}

#[test]
fn first_scoped_value_shadows_a_cached_missing_value() {
    let cx = counted_cx();
    assert_eq!(optional_database(&cx), None);

    let child = cx.with(Database("primary"));

    assert_eq!(optional_database(&child), Some("primary"));
    assert_eq!(calls(&cx).load(Ordering::SeqCst), 2);
}

#[test]
fn first_root_value_is_inherited_by_scopes() {
    let mut cx = counted_cx();
    assert_eq!(cx.insert(Database("primary")), None);

    assert_eq!(optional_database(&cx), Some("primary"));
    let child = cx.with(Config(1));

    assert_eq!(optional_database(&child), Some("primary"));
    assert_eq!(calls(&cx).load(Ordering::SeqCst), 1);
}

#[test]
fn root_value_after_a_missing_read_invalidates_the_cached_variant() {
    let mut cx = counted_cx();
    assert_eq!(optional_database(&cx), None);

    assert_eq!(cx.insert(Database("primary")), None);

    assert_eq!(optional_database(&cx), Some("primary"));
    assert_eq!(calls(&cx).load(Ordering::SeqCst), 2);
}

#[test]
fn unrelated_scoped_value_preserves_a_cached_missing_value() {
    let cx = counted_cx();
    assert_eq!(optional_database(&cx), None);

    let child = cx.with(Config(1));

    assert_eq!(optional_database(&child), None);
    assert_eq!(calls(&cx).load(Ordering::SeqCst), 1);
}

#[test]
fn replacing_a_root_value_returns_it_and_invalidates_its_cached_variant() {
    let mut cx = counted_cx();
    assert_eq!(cx.insert(Database("primary")), None);
    assert_eq!(optional_database(&cx), Some("primary"));

    assert_eq!(cx.insert(Database("replica")), Some(Database("primary")));

    assert_eq!(optional_database(&cx), Some("replica"));
    assert_eq!(calls(&cx).load(Ordering::SeqCst), 2);
}

#[test]
fn first_root_value_preserves_earlier_cached_missing_values() {
    let mut cx = counted_cx();
    {
        let child = cx.with(Config(1));
        assert_eq!(optional_config(&cx), None);
        assert_eq!(request_context::<Config>(&child), &Config(1));
    }

    assert_eq!(cx.insert(Database("primary")), None);

    assert_eq!(optional_config(&cx), None);
    assert_eq!(request_context::<Database>(&cx), &Database("primary"));
    assert_eq!(calls(&cx).load(Ordering::SeqCst), 1);
}

#[test]
fn get_mut_invalidates_only_after_a_successful_lookup() {
    let mut cx = CxTestBuilder::new()
        .app_context(AtomicUsize::new(0))
        .request_context(Config(1))
        .build();
    assert_eq!(optional_config(&cx), Some(1));

    cx.get_mut::<Config>().unwrap().0 = 42;

    assert_eq!(optional_config(&cx), Some(42));
    assert_eq!(optional_database(&cx), None);
    assert_eq!(cx.get_mut::<Database>(), None);
    assert_eq!(optional_database(&cx), None);
    assert_eq!(calls(&cx).load(Ordering::SeqCst), 3);
}

#[test]
fn child_scope_shadows_parent_without_changing_it() {
    let mut cx = Cx::default();
    cx.insert(Database("primary"));
    let child = cx.with(Database("replica"));

    assert_eq!(request_context::<Database>(&child), &Database("replica"));
    assert_eq!(request_context::<Database>(&cx), &Database("primary"));
}

#[test]
fn nearest_scoped_binding_wins() {
    let cx = Cx::default();
    let child = cx.with(Database("replica"));
    let grandchild = child.with(Database("archive"));

    assert_eq!(
        request_context::<Database>(&grandchild),
        &Database("archive")
    );
    assert_eq!(request_context::<Database>(&child), &Database("replica"));
}

#[test]
fn unrelated_values_are_inherited() {
    let cx = CxTestBuilder::new().request_context(Config(42)).build();
    let child = cx.with(Database("replica"));

    assert_eq!(request_context::<Config>(&child), &Config(42));
}

#[test]
fn dropping_a_scope_leaves_the_parent_unchanged() {
    let mut cx = Cx::default();
    {
        let child = cx.with(Database("replica"));
        assert_eq!(request_context::<Database>(&child), &Database("replica"));
    }

    assert_eq!(try_request_context::<Database>(&cx), None);
    cx.insert(Database("primary"));
    assert_eq!(request_context::<Database>(&cx), &Database("primary"));
}

#[test]
fn dropping_a_scope_drops_only_its_shadowing_binding() {
    let parent_drops = Arc::new(AtomicUsize::new(0));
    let child_drops = Arc::new(AtomicUsize::new(0));
    let cx = Cx::default();
    let parent = cx.with(DropCounter(parent_drops.clone()));

    {
        let child = parent.with(DropCounter(child_drops.clone()));
        assert!(Arc::ptr_eq(
            &request_context::<DropCounter>(&child).0,
            &child_drops,
        ));
    }

    assert_eq!(child_drops.load(Ordering::SeqCst), 1);
    assert_eq!(parent_drops.load(Ordering::SeqCst), 0);
    drop(parent);
    assert_eq!(parent_drops.load(Ordering::SeqCst), 1);
}

#[test]
fn with_values_installs_each_tuple_element() {
    let cx = Cx::default();
    let child = cx.with_values((Database("primary"), Config(42)));

    assert_eq!(request_context::<Database>(&child), &Database("primary"));
    assert_eq!(request_context::<Config>(&child), &Config(42));
}

#[test]
fn with_treats_a_tuple_as_one_binding() {
    let cx = Cx::default();
    let child = cx.with((Database("primary"), Config(42)));

    assert_eq!(
        request_context::<(Database, Config)>(&child),
        &(Database("primary"), Config(42))
    );
    assert_eq!(try_request_context::<Database>(&child), None);
}

#[test]
#[should_panic(expected = "duplicate value types")]
fn with_values_rejects_duplicate_types() {
    let cx = Cx::default();
    let _ = cx.with_values((Config(1), Config(2)));
}

#[test]
fn with_values_supports_arities_two_through_twelve() {
    let cx = Cx::default();
    let _ = cx.with_values((0u8, 1u16));
    let _ = cx.with_values((0u8, 1u16, 2u32));
    let _ = cx.with_values((0u8, 1u16, 2u32, 3u64));
    let _ = cx.with_values((0u8, 1u16, 2u32, 3u64, 4u128));
    let _ = cx.with_values((0u8, 1u16, 2u32, 3u64, 4u128, 5i8));
    let _ = cx.with_values((0u8, 1u16, 2u32, 3u64, 4u128, 5i8, 6i16));
    let _ = cx.with_values((0u8, 1u16, 2u32, 3u64, 4u128, 5i8, 6i16, 7i32));
    let _ = cx.with_values((0u8, 1u16, 2u32, 3u64, 4u128, 5i8, 6i16, 7i32, 8i64));
    let _ = cx.with_values((0u8, 1u16, 2u32, 3u64, 4u128, 5i8, 6i16, 7i32, 8i64, 9i128));
    let _ = cx.with_values((
        0u8, 1u16, 2u32, 3u64, 4u128, 5i8, 6i16, 7i32, 8i64, 9i128, 10usize,
    ));
    let _ = cx.with_values((
        0u8, 1u16, 2u32, 3u64, 4u128, 5i8, 6i16, 7i32, 8i64, 9i128, 10usize, 11isize,
    ));
}

#[test]
fn each_scope_has_a_fresh_id() {
    let cx = Cx::default();
    let child = cx.with(Config(1));
    let grandchild = child.with(Database("primary"));

    assert_ne!(cx.id(), child.id());
    assert_ne!(child.id(), grandchild.id());
}

#[test]
fn context_types_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<Cx>();
    assert_send_sync::<CxScope<'static>>();
}
