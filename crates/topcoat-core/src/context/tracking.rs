//! Tracking of request context reads, backing `#[memoize]`'s dependency
//! checks.

use std::{
    any::{Any, TypeId},
    sync::{Arc, Mutex},
};

use crate::context::{BindingId, RequestContext};

/// One observed request context read: the type that was looked up and the
/// identity of the binding it resolved to, or `None` if no value was
/// registered.
///
/// A read stays valid for a context as long as that context resolves the same
/// type to the same binding. Scopes only ever add or shadow bindings, so a
/// read is invalidated in exactly one of two ways: a present read by
/// shadowing the type, an absence read by registering it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextRead {
    type_id: TypeId,
    binding_id: Option<BindingId>,
}

impl ContextRead {
    /// Records what `context` currently resolves `T` to.
    pub(crate) fn observe<T>(context: &RequestContext) -> Self
    where
        T: Any,
    {
        let type_id = TypeId::of::<T>();
        Self {
            type_id,
            binding_id: context.binding_id(type_id),
        }
    }

    /// Returns whether `context` still resolves this read's type to the
    /// binding that was observed.
    pub(crate) fn matches(&self, context: &RequestContext) -> bool {
        context.binding_id(self.type_id) == self.binding_id
    }
}

/// Records the request context reads made through one tracked call.
///
/// A tracker is installed on a [`Cx`] handle with [`Cx::track`] and travels
/// with every handle derived from it, including clones moved to other
/// threads. Each read is checked against the scope the tracked call was
/// entered with: a read that resolves differently there went through a
/// binding the tracked body created itself, which its caller cannot see, so
/// it is not a dependency and is dropped.
///
/// [`Cx`]: crate::context::Cx
/// [`Cx::track`]: crate::context::Cx::track
#[derive(Debug)]
pub(crate) struct ContextTracker {
    /// The scope the tracked call was entered with.
    entry: Arc<RequestContext>,
    reads: Mutex<Vec<ContextRead>>,
}

impl ContextTracker {
    /// Creates a tracker for a call entered with the scope `entry`.
    pub(crate) fn new(entry: Arc<RequestContext>) -> Self {
        Self {
            entry,
            reads: Mutex::new(Vec::new()),
        }
    }

    /// Records that `T` was read through a handle whose scope is `context`,
    /// unless the read resolved through a binding the tracked call created
    /// itself.
    pub(crate) fn record<T>(&self, context: &RequestContext)
    where
        T: Any,
    {
        let read = ContextRead::observe::<T>(context);
        if !read.matches(&self.entry) {
            // The context value has been shadowed since tracking started,
            // so we do not need to track this particular read.
            return;
        }
        let mut reads = self.reads.lock().expect("context tracker lock poisoned");
        if !reads.contains(&read) {
            reads.push(read);
        }
    }

    /// Returns the reads recorded so far.
    // TODO: unused only until the memoize integration lands; remove with it.
    #[allow(dead_code)]
    pub(crate) fn reads(&self) -> Vec<ContextRead> {
        self.reads
            .lock()
            .expect("context tracker lock poisoned")
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{Cx, request_context, try_request_context};

    struct Session;
    struct Theme;

    /// Observes what `cx`'s scope currently resolves `T` to, for building
    /// expected reads.
    fn expected<T: Any>(cx: &Cx) -> ContextRead {
        ContextRead::observe::<T>(&cx.request_context)
    }

    #[test]
    fn a_present_read_matches_its_own_context() {
        let mut context = RequestContext::new();
        context.insert(Session);

        let read = ContextRead::observe::<Session>(&context);
        assert!(read.matches(&context));
    }

    #[test]
    fn shadowing_invalidates_a_present_read() {
        let mut context = RequestContext::new();
        context.insert(Session);
        let read = ContextRead::observe::<Session>(&context);

        let mut shadowed = context.clone();
        shadowed.insert(Session);

        assert!(read.matches(&context));
        assert!(!read.matches(&shadowed));
    }

    #[test]
    fn an_absence_read_matches_while_the_type_is_absent() {
        let context = RequestContext::new();
        let read = ContextRead::observe::<Session>(&context);
        assert!(read.matches(&context));
    }

    #[test]
    fn registering_the_type_invalidates_an_absence_read() {
        let context = RequestContext::new();
        let read = ContextRead::observe::<Session>(&context);

        let mut extended = context.clone();
        extended.insert(Session);

        assert!(read.matches(&context));
        assert!(!read.matches(&extended));
    }

    #[test]
    fn an_unrelated_binding_leaves_a_read_valid() {
        let mut context = RequestContext::new();
        context.insert(Session);
        let read = ContextRead::observe::<Session>(&context);

        let mut extended = context.clone();
        extended.insert(Theme);

        assert!(read.matches(&extended));
    }

    #[test]
    fn a_cloned_context_keeps_reads_valid() {
        let mut context = RequestContext::new();
        context.insert(Session);
        let read = ContextRead::observe::<Session>(&context);

        assert!(read.matches(&context.clone()));
    }

    #[test]
    fn a_present_read_is_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        let _: &Session = request_context(&child);

        assert_eq!(tracker.reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn an_absence_read_is_recorded() {
        let cx = Cx::default();
        let (child, tracker) = cx.track();

        assert!(try_request_context::<Session>(&child).is_none());
        assert_eq!(tracker.reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn a_repeated_read_is_recorded_once() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        let _ = try_request_context::<Session>(&child);
        let _ = try_request_context::<Session>(&child);

        assert_eq!(tracker.reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn distinct_reads_are_all_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        let _ = try_request_context::<Session>(&child);
        let _ = try_request_context::<Theme>(&child);

        assert_eq!(
            tracker.reads(),
            [expected::<Session>(&cx), expected::<Theme>(&cx)]
        );
    }

    #[test]
    fn an_inherited_read_through_a_derived_scope_is_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();
        let derived = child.with(Theme);

        let _ = try_request_context::<Session>(&derived);

        assert_eq!(tracker.reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn a_read_of_an_internal_binding_is_not_recorded() {
        let cx = Cx::default();
        let (child, tracker) = cx.track();
        let derived = child.with(Theme);

        let _ = try_request_context::<Theme>(&derived);

        assert_eq!(tracker.reads(), []);
    }

    #[test]
    fn a_read_of_an_internal_shadowing_binding_is_not_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();
        let derived = child.with(Session);

        let _ = try_request_context::<Session>(&derived);

        assert_eq!(tracker.reads(), []);
    }

    #[test]
    fn a_read_through_the_untracked_original_is_not_recorded() {
        let cx = Cx::default().with(Session);
        let (_child, tracker) = cx.track();

        let _ = try_request_context::<Session>(&cx);

        assert_eq!(tracker.reads(), []);
    }

    #[test]
    fn a_read_through_a_clone_of_the_tracked_handle_is_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        let _ = try_request_context::<Session>(&child.clone());

        assert_eq!(tracker.reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn a_read_on_another_thread_is_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        std::thread::spawn(move || {
            let _ = try_request_context::<Session>(&child);
        })
        .join()
        .expect("tracked thread panicked");

        assert_eq!(tracker.reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn a_nested_track_replaces_the_inherited_tracker() {
        let cx = Cx::default().with(Session);
        let (child, outer) = cx.track();
        let (nested, inner) = child.track();

        let _ = try_request_context::<Session>(&nested);

        assert_eq!(outer.reads(), []);
        assert_eq!(inner.reads(), [expected::<Session>(&cx)]);
    }
}
