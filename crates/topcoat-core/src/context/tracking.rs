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
    /// Records that `T` resolved to `binding_id`, or to nothing at all.
    pub(crate) fn new<T>(binding_id: Option<BindingId>) -> Self
    where
        T: Any,
    {
        Self {
            type_id: TypeId::of::<T>(),
            binding_id,
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

    /// Records `read`, unless it resolved through a binding the tracked call
    /// created itself.
    pub(crate) fn record(&self, read: ContextRead) {
        self.merge(&[read]);
    }

    /// Records every read in `reads` that passes the entry filter, skipping
    /// reads already recorded.
    ///
    /// Replaying a nested tracked call's reads through this re-expresses them
    /// relative to this tracker's own entry scope: a read whose value was
    /// shadowed after tracking started resolves through a binding the tracked
    /// call created itself, so it is not a dependency and is dropped, even
    /// where it was a genuine dependency of the nested call.
    pub(crate) fn merge(&self, reads: &[ContextRead]) {
        // A body that read no request context has nothing to contribute, and is
        // common enough to be worth keeping off the lock.
        if reads.is_empty() {
            return;
        }
        let mut own = self.reads.lock().expect("context tracker lock poisoned");
        for read in reads {
            if read.matches(&self.entry) && !own.contains(read) {
                own.push(*read);
            }
        }
    }

    /// Returns the reads recorded so far, leaving the tracker empty.
    ///
    /// Called once, when the tracked call is done and its reads are handed to
    /// the variant they belong to, so the recorded reads are moved out rather
    /// than copied.
    pub(crate) fn take_reads(&self) -> Vec<ContextRead> {
        std::mem::take(&mut *self.reads.lock().expect("context tracker lock poisoned"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{Cx, request_context, try_request_context};

    struct Session;
    struct Theme;

    /// Records what `context` currently resolves `T` to.
    fn observe<T: Any>(context: &RequestContext) -> ContextRead {
        ContextRead::new::<T>(context.binding_id(TypeId::of::<T>()))
    }

    /// Observes what `cx`'s scope currently resolves `T` to, for building
    /// expected reads.
    fn expected<T: Any>(cx: &Cx) -> ContextRead {
        observe::<T>(&cx.request_context)
    }

    #[test]
    fn a_present_read_matches_its_own_context() {
        let mut context = RequestContext::new();
        context.insert(Session);

        let read = observe::<Session>(&context);
        assert!(read.matches(&context));
    }

    #[test]
    fn shadowing_invalidates_a_present_read() {
        let mut context = RequestContext::new();
        context.insert(Session);
        let read = observe::<Session>(&context);

        let mut shadowed = context.clone();
        shadowed.insert(Session);

        assert!(read.matches(&context));
        assert!(!read.matches(&shadowed));
    }

    #[test]
    fn an_absence_read_matches_while_the_type_is_absent() {
        let context = RequestContext::new();
        let read = observe::<Session>(&context);
        assert!(read.matches(&context));
    }

    #[test]
    fn registering_the_type_invalidates_an_absence_read() {
        let context = RequestContext::new();
        let read = observe::<Session>(&context);

        let mut extended = context.clone();
        extended.insert(Session);

        assert!(read.matches(&context));
        assert!(!read.matches(&extended));
    }

    #[test]
    fn an_unrelated_binding_leaves_a_read_valid() {
        let mut context = RequestContext::new();
        context.insert(Session);
        let read = observe::<Session>(&context);

        let mut extended = context.clone();
        extended.insert(Theme);

        assert!(read.matches(&extended));
    }

    #[test]
    fn a_cloned_context_keeps_reads_valid() {
        let mut context = RequestContext::new();
        context.insert(Session);
        let read = observe::<Session>(&context);

        assert!(read.matches(&context.clone()));
    }

    #[test]
    fn a_present_read_is_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        let _: &Session = request_context(&child);

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn an_absence_read_is_recorded() {
        let cx = Cx::default();
        let (child, tracker) = cx.track();

        assert!(try_request_context::<Session>(&child).is_none());
        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn a_repeated_read_is_recorded_once() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        let _ = try_request_context::<Session>(&child);
        let _ = try_request_context::<Session>(&child);

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn distinct_reads_are_all_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        let _ = try_request_context::<Session>(&child);
        let _ = try_request_context::<Theme>(&child);

        assert_eq!(
            tracker.take_reads(),
            [expected::<Session>(&cx), expected::<Theme>(&cx)]
        );
    }

    #[test]
    fn an_inherited_read_through_a_derived_scope_is_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();
        let derived = child.with(Theme);

        let _ = try_request_context::<Session>(&derived);

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn a_read_of_an_internal_binding_is_not_recorded() {
        let cx = Cx::default();
        let (child, tracker) = cx.track();
        let derived = child.with(Theme);

        let _ = try_request_context::<Theme>(&derived);

        assert_eq!(tracker.take_reads(), []);
    }

    #[test]
    fn a_read_of_an_internal_shadowing_binding_is_not_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();
        let derived = child.with(Session);

        let _ = try_request_context::<Session>(&derived);

        assert_eq!(tracker.take_reads(), []);
    }

    #[test]
    fn a_read_through_the_untracked_original_is_not_recorded() {
        let cx = Cx::default().with(Session);
        let (_child, tracker) = cx.track();

        let _ = try_request_context::<Session>(&cx);

        assert_eq!(tracker.take_reads(), []);
    }

    #[test]
    fn a_read_through_a_clone_of_the_tracked_handle_is_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        let _ = try_request_context::<Session>(&child.clone());

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
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

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn merge_applies_the_entry_filter_and_deduplicates() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();
        let scoped = child.with(Theme);
        let inherited = expected::<Session>(&cx);
        let internal = expected::<Theme>(&scoped);

        tracker.merge(&[inherited, internal, inherited]);

        assert_eq!(tracker.take_reads(), [inherited]);
    }

    #[test]
    fn a_nested_track_replaces_the_inherited_tracker() {
        let cx = Cx::default().with(Session);
        let (child, outer) = cx.track();
        let (nested, inner) = child.track();

        let _ = try_request_context::<Session>(&nested);

        assert_eq!(outer.take_reads(), []);
        assert_eq!(inner.take_reads(), [expected::<Session>(&cx)]);
    }
}
