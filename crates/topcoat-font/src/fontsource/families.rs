//! Generated metadata for every font family in the vendored Fontsource
//! catalog.
//!
//! Each family is a [`Family`] constant named after its id in
//! `SCREAMING_SNAKE_CASE` (e.g. [`ROBOTO`], [`JETBRAINS_MONO`]). Iterate the
//! whole catalog through [`ALL`], or look a family up by id with
//! [`Family::by_id`].

use crate::{UnicodeRange, UnicodeRanges};

use super::{Family, Style, Subset};

include!(concat!(env!("OUT_DIR"), "/families.rs"));
