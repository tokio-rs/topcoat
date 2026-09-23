//! Every font family in the Fontsource catalog.
//!
//! Each family is a [`Family`] constant named after its id in
//! `SCREAMING_SNAKE_CASE`, such as [`ROBOTO`] or [`JETBRAINS_MONO`]. An id
//! that starts with a digit gets a `_` prefix. Iterate over the whole catalog
//! with [`ALL`], or look a family up by id with [`Family::by_id`].

use super::{Family, Style, Subset};
use crate::{UnicodeRange, UnicodeRanges};

include!(concat!(env!("OUT_DIR"), "/families.rs"));
