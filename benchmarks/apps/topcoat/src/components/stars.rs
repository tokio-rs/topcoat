use topcoat::{
    Result,
    context::Cx,
    view::{AttributeValueViewParts, StaticClass, View, class, component, view},
};

/// The size of a star row.
#[derive(Clone, Copy)]
pub enum StarSize {
    Small,
    Large,
}

impl StarSize {
    fn class(self, filled: bool) -> StaticClass {
        match (self, filled) {
            (Self::Small, true) => class!("size-4 fill-amber-400"),
            (Self::Small, false) => class!("size-4 fill-stone-300"),
            (Self::Large, true) => class!("size-5 fill-amber-400"),
            (Self::Large, false) => class!("size-5 fill-stone-300"),
        }
    }
}

/// A row of five stars with `filled` of them highlighted.
///
/// The row is a single image to assistive technology, labelled with `rating`.
#[component]
pub async fn stars(
    cx: &Cx,
    filled: u32,
    rating: impl AttributeValueViewParts + Send,
    size: StarSize,
) -> Result<impl View> {
    Ok(view! {
        <span
            class="flex items-center gap-0.5"
            role="img"
            aria-label=(("Rated ", rating, " out of 5"))
        >
            for star in 0..5 {
                <svg viewBox="0 0 20 20" class=(size.class(star < filled))>
                    <path d="M10 1l3 6 6 1-4 4 1 6-6-3-6 3 1-6-4-4 6-1z"></path>
                </svg>
            }
        </span>
    })
}

/// The number of filled stars for a rating in tenths: the rating rounded half up.
pub fn filled_stars(rating_tenths: u32) -> u32 {
    (rating_tenths + 5) / 10
}
