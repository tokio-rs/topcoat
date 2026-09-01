use std::{cell::Cell, fmt};

/// Identifies a region within the output of one root view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionId(u64);

impl RegionId {
    /// Allocates the next id of the enclosing [`RegionScope`].
    ///
    /// # Panics
    ///
    /// Panics if no scope is active on the current thread.
    pub(crate) fn next() -> Self {
        let id = NEXT_REGION
            .get()
            .expect("tried to allocate a region id outside of a `RegionScope`");
        NEXT_REGION.set(Some(id + 1));
        Self(id)
    }

    /// Returns the id as its numeric value.
    #[must_use]
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl fmt::Display for RegionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Numbers the regions allocated below one root view.
///
/// The outermost scope of a poll installs its counter for the poll's
/// duration, and every [`RegionId::next`] call below it draws from that
/// counter. A scope entered inside another leaves the enclosing counter in
/// place, so ids stay unique across everything one root view produces while
/// distinct roots each count from the start.
pub(crate) struct RegionScope<'a> {
    /// The installing scope's counter, written back on exit. `None` for a
    /// scope entered inside another, which defers to the enclosing one.
    slot: Option<&'a mut u64>,
}

impl<'a> RegionScope<'a> {
    pub(crate) fn new(counter: &'a mut u64) -> Self {
        if NEXT_REGION.get().is_some() {
            return Self { slot: None };
        }
        NEXT_REGION.set(Some(*counter));
        Self {
            slot: Some(counter),
        }
    }
}

impl Drop for RegionScope<'_> {
    fn drop(&mut self) {
        if let Some(counter) = self.slot.take() {
            *counter = NEXT_REGION
                .take()
                .expect("the region counter was cleared inside a `RegionScope`");
        }
    }
}

thread_local! {
    /// The next region id of the innermost installed [`RegionScope`].
    static NEXT_REGION: Cell<Option<u64>> = const { Cell::new(None) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scope_issues_sequential_ids_and_keeps_its_count() {
        let mut counter = 1;
        {
            let _regions = RegionScope::new(&mut counter);
            assert_eq!(RegionId::next(), RegionId(1));
            assert_eq!(RegionId::next(), RegionId(2));
        }
        assert_eq!(counter, 3);

        let _regions = RegionScope::new(&mut counter);
        assert_eq!(RegionId::next(), RegionId(3));
    }

    #[test]
    fn a_nested_scope_defers_to_the_enclosing_one() {
        let mut outer = 1;
        let mut inner = 1;
        {
            let _outer = RegionScope::new(&mut outer);
            assert_eq!(RegionId::next(), RegionId(1));
            {
                let _inner = RegionScope::new(&mut inner);
                assert_eq!(RegionId::next(), RegionId(2));
            }
            assert_eq!(RegionId::next(), RegionId(3));
        }
        assert_eq!(outer, 4);
        assert_eq!(inner, 1);
    }

    #[test]
    fn distinct_scopes_each_count_from_the_start() {
        for _ in 0..2 {
            let mut counter = 1;
            let _regions = RegionScope::new(&mut counter);
            assert_eq!(RegionId::next(), RegionId(1));
        }
    }

    #[test]
    #[should_panic = "outside of a `RegionScope`"]
    fn allocating_outside_a_scope_panics() {
        let _ = RegionId::next();
    }
}
