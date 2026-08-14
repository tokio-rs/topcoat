use std::{borrow::Cow, collections::HashMap};

use crate::{IntoPath, Path, RouteId};

/// The mounted path of every registered handler, keyed by [`RouteId`].
///
/// The router builds the table when it is finalized and registers it on the
/// app context, where a handler's id resolves back to the path it serves.
#[derive(Debug, Default)]
pub struct RouteTable {
    paths: HashMap<RouteId, Cow<'static, Path>>,
}

impl RouteTable {
    /// Creates an empty table.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `path` as the mounted path of the handler `id`.
    ///
    /// # Panics
    ///
    /// Panics if the path is a string that is not a well-formed path, or if
    /// a path is already recorded for `id`.
    #[track_caller]
    pub fn insert(&mut self, id: RouteId, path: impl IntoPath) {
        let path = path.into_path();
        assert!(
            self.paths.insert(id, path).is_none(),
            "duplicate route table entry for a handler id"
        );
    }

    /// Returns the mounted path of the handler `id`, or `None` if it is not
    /// registered.
    #[must_use]
    pub fn get(&self, id: RouteId) -> Option<&Path> {
        self.paths.get(&id).map(|path| &**path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_resolves_paths() {
        let id = RouteId::new();
        let mut table = RouteTable::new();
        table.insert(id, "/posts/{post_id}");
        assert_eq!(table.get(id), Some(Path::new("/posts/{post_id}")));
        assert_eq!(table.get(RouteId::new()), None);
    }

    #[test]
    #[should_panic(expected = "duplicate route table entry")]
    fn a_duplicate_entry_panics() {
        let id = RouteId::new();
        let mut table = RouteTable::new();
        table.insert(id, "/a");
        table.insert(id, "/b");
    }
}
