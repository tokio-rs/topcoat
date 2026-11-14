use std::fmt::Display;

use maud::{Markup, html};

const STAR_PATH: &str = "M10 1l3 6 6 1-4 4 1 6-6-3-6 3 1-6-4-4 6-1z";

/// The size of a star row.
#[derive(Clone, Copy)]
pub enum StarSize {
    Small,
    Large,
}

impl StarSize {
    fn class(self, filled: bool) -> &'static str {
        match (self, filled) {
            (Self::Small, true) => "size-4 fill-amber-400",
            (Self::Small, false) => "size-4 fill-stone-300",
            (Self::Large, true) => "size-5 fill-amber-400",
            (Self::Large, false) => "size-5 fill-stone-300",
        }
    }
}

/// A row of five stars with `filled` of them highlighted.
///
/// The row is a single image to assistive technology, labelled with `rating`.
pub fn stars(filled: u32, rating: impl Display, size: StarSize) -> Markup {
    html! {
        span class="flex items-center gap-0.5" role="img" aria-label={ "Rated " (rating) " out of 5" } {
            @for star in 0..5 {
                svg viewBox="0 0 20 20" class=(size.class(star < filled)) {
                    path d=(STAR_PATH) {}
                }
            }
        }
    }
}

/// The number of filled stars for a rating in tenths: the rating rounded half up.
pub fn filled_stars(rating_tenths: u32) -> u32 {
    (rating_tenths + 5) / 10
}
