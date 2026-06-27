//! Generated metadata for every font family in the vendored Fontsource
//! catalog.
//!
//! Each family is a [`Family`] constant named after its id in
//! `SCREAMING_SNAKE_CASE` (e.g. [`ROBOTO`], [`JETBRAINS_MONO`]). Iterate the
//! whole catalog through [`ALL`], or look a family up by id with [`by_id`].

use crate::{Family, Style, Subset};

include!(concat!(env!("OUT_DIR"), "/families.rs"));

/// Look up a family by its Fontsource [`id`](Family::id), e.g.
/// `"roboto"`. Returns `None` if no such family is in the vendored catalog.
#[must_use]
pub fn by_id(id: &str) -> Option<&'static Family> {
    ALL.binary_search_by(|f| f.id.cmp(id)).ok().map(|i| ALL[i])
}
