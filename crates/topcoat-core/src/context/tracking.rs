//! Tracking of request context reads, backing `#[memoize]`'s dependency
//! checks.

use std::any::{Any, TypeId};

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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub(crate) fn matches(&self, context: &RequestContext) -> bool {
        context.binding_id(self.type_id) == self.binding_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Session;
    struct Theme;

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
}
