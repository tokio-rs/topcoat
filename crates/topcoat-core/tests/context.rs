use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use topcoat_core::context::{Cx, CxScope, CxTestBuilder, request_context, try_request_context};

#[derive(Debug, PartialEq)]
struct Database(&'static str);

#[derive(Debug, PartialEq)]
struct Config(u32);

struct DropCounter(Arc<AtomicUsize>);

impl Drop for DropCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
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
