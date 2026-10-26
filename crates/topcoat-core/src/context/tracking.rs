//! Records context reads so `#[memoize]` can decide when to reuse a result.

use std::{
    any::{Any, TypeId},
    sync::{Arc, Mutex},
};

use crate::context::{BindingId, Scope};

/// A saved context lookup, including its type and the binding ID it found.
/// A missing value is recorded as `None`.
///
/// A saved read matches a context if looking up its type finds the same
/// binding ID. Replacing the value breaks that match. If the read found
/// nothing, adding a value of that type breaks the match too.
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

    /// Checks whether `scope` would return the same binding as this read.
    pub(crate) fn matches(&self, scope: &Scope) -> bool {
        scope.binding_id(self.type_id) == self.binding_id
    }
}

/// Collects context reads for one function call.
///
/// [`Cx::track`] attaches the tracker to a handle. Children and clones keep
/// it, including clones sent to other threads. The tracker compares reads
/// with the context at the start of the call. It ignores reads of values
/// added by the function itself, since those do not depend on the caller.
///
/// [`Cx`]: crate::context::Cx
/// [`Cx::track`]: crate::context::Cx::track
#[derive(Debug)]
pub(crate) struct ContextTracker {
    /// The scope the tracked call was entered with.
    entry: Arc<Scope>,
    reads: Mutex<Vec<ContextRead>>,
}

impl ContextTracker {
    /// Creates a tracker for a call entered with the scope `entry`.
    pub(crate) fn new(entry: Arc<Scope>) -> Self {
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
        // Skip the lock when the function read no context values.
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
    use crate::context::{Cx, context, try_context};

    struct Session;
    struct Theme;

    /// Observes what `cx`'s scope currently resolves `T` to, for building
    /// expected reads.
    fn expected<T: Any>(cx: &Cx) -> ContextRead {
        ContextRead::new::<T>(cx.scope().binding_id(TypeId::of::<T>()))
    }

    #[test]
    fn a_present_read_matches_its_own_scope() {
        let cx = Cx::default().with(Session);

        let read = expected::<Session>(&cx);
        assert!(read.matches(cx.scope()));
    }

    #[test]
    fn shadowing_invalidates_a_present_read() {
        let cx = Cx::default().with(Session);
        let read = expected::<Session>(&cx);

        let shadowed = cx.with(Session);

        assert!(read.matches(cx.scope()));
        assert!(!read.matches(shadowed.scope()));
    }

    #[test]
    fn an_absence_read_matches_while_the_type_is_absent() {
        let cx = Cx::default();
        let read = expected::<Session>(&cx);
        assert!(read.matches(cx.scope()));
    }

    #[test]
    fn registering_the_type_invalidates_an_absence_read() {
        let cx = Cx::default();
        let read = expected::<Session>(&cx);

        let extended = cx.with(Session);

        assert!(read.matches(cx.scope()));
        assert!(!read.matches(extended.scope()));
    }

    #[test]
    fn an_unrelated_binding_leaves_a_read_valid() {
        let cx = Cx::default().with(Session);
        let read = expected::<Session>(&cx);

        let extended = cx.with(Theme);

        assert!(read.matches(extended.scope()));
    }

    #[test]
    fn a_present_read_is_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        let _: &Session = context(&child);

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn an_absence_read_is_recorded() {
        let cx = Cx::default();
        let (child, tracker) = cx.track();

        assert!(try_context::<Session>(&child).is_none());
        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn a_repeated_read_is_recorded_once() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        let _ = try_context::<Session>(&child);
        let _ = try_context::<Session>(&child);

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn distinct_reads_are_all_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        let _ = try_context::<Session>(&child);
        let _ = try_context::<Theme>(&child);

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

        let _ = try_context::<Session>(&derived);

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn a_read_of_an_internal_binding_is_not_recorded() {
        let cx = Cx::default();
        let (child, tracker) = cx.track();
        let derived = child.with(Theme);

        let _ = try_context::<Theme>(&derived);

        assert_eq!(tracker.take_reads(), []);
    }

    #[test]
    fn a_read_of_an_internal_shadowing_binding_is_not_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();
        let derived = child.with(Session);

        let _ = try_context::<Session>(&derived);

        assert_eq!(tracker.take_reads(), []);
    }

    #[test]
    fn a_read_through_the_untracked_original_is_not_recorded() {
        let cx = Cx::default().with(Session);
        let (_child, tracker) = cx.track();

        let _ = try_context::<Session>(&cx);

        assert_eq!(tracker.take_reads(), []);
    }

    #[test]
    fn a_read_through_a_clone_of_the_tracked_handle_is_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        let _ = try_context::<Session>(&child.clone());

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn a_read_on_another_thread_is_recorded() {
        let cx = Cx::default().with(Session);
        let (child, tracker) = cx.track();

        std::thread::spawn(move || {
            let _ = try_context::<Session>(&child);
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

        let _ = try_context::<Session>(&nested);

        assert_eq!(outer.take_reads(), []);
        assert_eq!(inner.take_reads(), [expected::<Session>(&cx)]);
    }
}
