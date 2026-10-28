//! Records context reads so `#[memoize]` can decide when to reuse a result.

use std::{
    any::{Any, TypeId},
    sync::{Arc, Mutex},
};

use crate::context::{BindingId, Scope};

/// A saved context read, including its type and what it found.
///
/// A saved read matches a context if reading its type there finds the same
/// thing. Shadowing the value breaks that match. If the read found nothing,
/// adding a value of that type breaks the match too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextRead {
    type_id: TypeId,
    found: Found,
}

/// What a saved read found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Found {
    /// The closest value of the type, or `None` if there was none.
    Closest(Option<BindingId>),
    /// Every value of the type.
    ///
    /// `request` is the newest request value, whose ID identifies all
    /// request values of the type. `sealed` is the newest sealed layer,
    /// whose ID identifies all sealed layers.
    All {
        request: Option<BindingId>,
        sealed: Option<BindingId>,
    },
}

impl ContextRead {
    /// Records that `T` resolved to `binding_id`, or to nothing at all.
    pub(crate) fn new<T>(binding_id: Option<BindingId>) -> Self
    where
        T: Any,
    {
        Self {
            type_id: TypeId::of::<T>(),
            found: Found::Closest(binding_id),
        }
    }

    /// Records that every value of `T` in `scope` was read.
    pub(crate) fn all<T>(scope: &Scope) -> Self
    where
        T: Any,
    {
        Self::all_of(TypeId::of::<T>(), scope)
    }

    /// Records that every value of `type_id` in `scope` was read.
    fn all_of(type_id: TypeId, scope: &Scope) -> Self {
        Self {
            type_id,
            found: Found::All {
                request: scope.request_binding_id(type_id),
                sealed: scope.sealed_id(),
            },
        }
    }

    /// Checks whether reading this read's type in `scope` would find the
    /// same values.
    pub(crate) fn matches(&self, scope: &Scope) -> bool {
        match self.found {
            Found::Closest(binding_id) => scope.binding_id(self.type_id) == binding_id,
            Found::All { .. } => Self::all_of(self.type_id, scope) == *self,
        }
    }

    /// Re-expresses this read relative to `entry`, the scope a tracked call
    /// started with. Returns `None` if the read does not depend on `entry`.
    ///
    /// A closest read that `entry` does not match found a value the call
    /// added itself, so it is dropped. A read of every value is kept but
    /// re-read from `entry`. Values the call added only stack on top of the
    /// values of `entry`, so the walk also visited every value of `entry`.
    fn relative_to(&self, entry: &Scope) -> Option<Self> {
        match self.found {
            Found::Closest(_) => self.matches(entry).then_some(*self),
            Found::All { .. } => Some(Self::all_of(self.type_id, entry)),
        }
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

    /// Records `read`, unless it only found a binding the tracked call
    /// created itself.
    pub(crate) fn record(&self, read: ContextRead) {
        self.merge(&[read]);
    }

    /// Records every read in `reads` relative to the entry scope, skipping
    /// reads already recorded.
    ///
    /// Replaying a nested tracked call's reads through this re-expresses them
    /// relative to this tracker's own entry scope: a closest read whose value
    /// was shadowed after tracking started resolves through a binding the
    /// tracked call created itself, so it is not a dependency and is dropped,
    /// even where it was a genuine dependency of the nested call. A read of
    /// every value is re-read from the entry scope instead, since the walk
    /// continues into the values of the entry scope.
    pub(crate) fn merge(&self, reads: &[ContextRead]) {
        // Skip the lock when the function read no context values.
        if reads.is_empty() {
            return;
        }
        let mut own = self.reads.lock().expect("context tracker lock poisoned");
        for read in reads {
            if let Some(read) = read.relative_to(&self.entry)
                && !own.contains(&read)
            {
                own.push(read);
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
    use crate::context::{Cx, context, iter_context, try_context};

    struct Session;
    struct Theme;

    /// Observes what `cx`'s scope currently resolves `T` to, for building
    /// expected reads.
    fn expected<T: Any>(cx: &Cx) -> ContextRead {
        ContextRead::new::<T>(cx.scope().binding_id(TypeId::of::<T>()))
    }

    #[test]
    fn a_present_read_matches_its_own_scope() {
        let cx = Cx::new().with(Session);

        let read = expected::<Session>(&cx);
        assert!(read.matches(cx.scope()));
    }

    #[test]
    fn shadowing_invalidates_a_present_read() {
        let cx = Cx::new().with(Session);
        let read = expected::<Session>(&cx);

        let shadowed = cx.with(Session);

        assert!(read.matches(cx.scope()));
        assert!(!read.matches(shadowed.scope()));
    }

    #[test]
    fn an_absence_read_matches_while_the_type_is_absent() {
        let cx = Cx::new();
        let read = expected::<Session>(&cx);
        assert!(read.matches(cx.scope()));
    }

    #[test]
    fn registering_the_type_invalidates_an_absence_read() {
        let cx = Cx::new();
        let read = expected::<Session>(&cx);

        let extended = cx.with(Session);

        assert!(read.matches(cx.scope()));
        assert!(!read.matches(extended.scope()));
    }

    #[test]
    fn an_unrelated_binding_leaves_a_read_valid() {
        let cx = Cx::new().with(Session);
        let read = expected::<Session>(&cx);

        let extended = cx.with(Theme);

        assert!(read.matches(extended.scope()));
    }

    #[test]
    fn a_present_read_is_recorded() {
        let cx = Cx::new().with(Session);
        let (child, tracker) = cx.track();

        let _: &Session = context(&child);

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn an_absence_read_is_recorded() {
        let cx = Cx::new();
        let (child, tracker) = cx.track();

        assert!(try_context::<Session>(&child).is_none());
        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn a_repeated_read_is_recorded_once() {
        let cx = Cx::new().with(Session);
        let (child, tracker) = cx.track();

        let _ = try_context::<Session>(&child);
        let _ = try_context::<Session>(&child);

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn distinct_reads_are_all_recorded() {
        let cx = Cx::new().with(Session);
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
        let cx = Cx::new().with(Session);
        let (child, tracker) = cx.track();
        let derived = child.with(Theme);

        let _ = try_context::<Session>(&derived);

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn a_read_of_an_internal_binding_is_not_recorded() {
        let cx = Cx::new();
        let (child, tracker) = cx.track();
        let derived = child.with(Theme);

        let _ = try_context::<Theme>(&derived);

        assert_eq!(tracker.take_reads(), []);
    }

    #[test]
    fn a_read_of_an_internal_shadowing_binding_is_not_recorded() {
        let cx = Cx::new().with(Session);
        let (child, tracker) = cx.track();
        let derived = child.with(Session);

        let _ = try_context::<Session>(&derived);

        assert_eq!(tracker.take_reads(), []);
    }

    #[test]
    fn a_read_through_the_untracked_original_is_not_recorded() {
        let cx = Cx::new().with(Session);
        let (_child, tracker) = cx.track();

        let _ = try_context::<Session>(&cx);

        assert_eq!(tracker.take_reads(), []);
    }

    #[test]
    fn a_read_through_a_clone_of_the_tracked_handle_is_recorded() {
        let cx = Cx::new().with(Session);
        let (child, tracker) = cx.track();

        let _ = try_context::<Session>(&child.clone());

        assert_eq!(tracker.take_reads(), [expected::<Session>(&cx)]);
    }

    #[test]
    fn a_read_on_another_thread_is_recorded() {
        let cx = Cx::new().with(Session);
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
        let cx = Cx::new().with(Session);
        let (child, tracker) = cx.track();
        let scoped = child.with(Theme);
        let inherited = expected::<Session>(&cx);
        let internal = expected::<Theme>(&scoped);

        tracker.merge(&[inherited, internal, inherited]);

        assert_eq!(tracker.take_reads(), [inherited]);
    }

    #[test]
    fn shadowing_invalidates_an_iteration_read() {
        let cx = Cx::new().with(Session);
        let read = ContextRead::all::<Session>(cx.scope());

        let shadowed = cx.with(Session);

        assert!(read.matches(cx.scope()));
        assert!(!read.matches(shadowed.scope()));
    }

    #[test]
    fn an_unrelated_binding_leaves_an_iteration_read_valid() {
        let cx = Cx::new().with(Session);
        let read = ContextRead::all::<Session>(cx.scope());

        let extended = cx.with(Theme);

        assert!(read.matches(extended.scope()));
    }

    #[test]
    fn other_sealed_layers_invalidate_an_iteration_read() {
        let cx = Cx::new().with_shared(Arc::default());
        let read = ContextRead::all::<Session>(cx.scope());

        assert!(!read.matches(Cx::new().with_shared(Arc::default()).scope()));
    }

    #[test]
    fn an_iteration_read_is_recorded() {
        let cx = Cx::new().with(Session);
        let (child, tracker) = cx.track();

        let _ = iter_context::<Session>(&child).count();

        assert_eq!(
            tracker.take_reads(),
            [ContextRead::all::<Session>(cx.scope())]
        );
    }

    #[test]
    fn an_iteration_through_an_internal_binding_is_recorded_from_the_entry() {
        let cx = Cx::new().with(Session);
        let (child, tracker) = cx.track();
        let derived = child.with(Session);

        let _ = iter_context::<Session>(&derived).count();

        assert_eq!(
            tracker.take_reads(),
            [ContextRead::all::<Session>(cx.scope())]
        );
    }

    #[test]
    fn closest_and_iteration_reads_of_one_type_are_both_recorded() {
        let cx = Cx::new().with(Session);
        let (child, tracker) = cx.track();

        let _ = try_context::<Session>(&child);
        let _ = iter_context::<Session>(&child).count();

        assert_eq!(
            tracker.take_reads(),
            [
                expected::<Session>(&cx),
                ContextRead::all::<Session>(cx.scope())
            ]
        );
    }

    #[test]
    fn a_nested_track_replaces_the_inherited_tracker() {
        let cx = Cx::new().with(Session);
        let (child, outer) = cx.track();
        let (nested, inner) = child.track();

        let _ = try_context::<Session>(&nested);

        assert_eq!(outer.take_reads(), []);
        assert_eq!(inner.take_reads(), [expected::<Session>(&cx)]);
    }
}
