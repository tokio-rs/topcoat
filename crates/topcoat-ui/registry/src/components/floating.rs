use topcoat::view::{Class, StaticClass, class};

/// The side of its anchor that a floating panel opens on.
///
/// [`Default`] is `FloatingSide::Bottom`, used when no side is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FloatingSide {
    /// Above the anchor.
    Top,
    /// To the right of the anchor.
    Right,
    /// Below the anchor.
    #[default]
    Bottom,
    /// To the left of the anchor.
    Left,
}

impl FloatingSide {
    /// Classes for the gap to the anchor and the positions to try when the panel would
    /// overflow the viewport. The panel first moves to the opposite side, then to the
    /// other alignment. Panels beside the anchor finally try above and below it.
    fn classes(self) -> StaticClass {
        match self {
            Self::Top => class!(
                "mb-1 [position-try-fallbacks:flip-block,flip-inline,flip-block_flip-inline]",
            ),
            Self::Right => class!(
                "ml-1 [position-try-fallbacks:flip-inline,flip-block,flip-inline_flip-block,",
                "top_span-left,bottom_span-left]",
            ),
            Self::Bottom => class!(
                "mt-1 [position-try-fallbacks:flip-block,flip-inline,flip-block_flip-inline]",
            ),
            Self::Left => class!(
                "mr-1 [position-try-fallbacks:flip-inline,flip-block,flip-inline_flip-block,",
                "top_span-right,bottom_span-right]",
            ),
        }
    }
}

/// How a floating panel lines up with its anchor along the side it opens on.
///
/// [`Default`] is `FloatingAlign::Center`, used when no alignment is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FloatingAlign {
    /// Lines up with the anchor's left or top edge.
    Start,
    /// Centers on the anchor.
    #[default]
    Center,
    /// Lines up with the anchor's right or bottom edge.
    End,
}

impl FloatingAlign {
    /// The `position-area` class that places the panel on `side` with this alignment.
    fn area(self, side: FloatingSide) -> StaticClass {
        match (side, self) {
            (FloatingSide::Top, Self::Start) => class!("[position-area:top_span-right]"),
            (FloatingSide::Top, Self::Center) => class!("[position-area:top]"),
            (FloatingSide::Top, Self::End) => class!("[position-area:top_span-left]"),
            (FloatingSide::Right, Self::Start) => class!("[position-area:right_span-bottom]"),
            (FloatingSide::Right, Self::Center) => class!("[position-area:right]"),
            (FloatingSide::Right, Self::End) => class!("[position-area:right_span-top]"),
            (FloatingSide::Bottom, Self::Start) => class!("[position-area:bottom_span-right]"),
            (FloatingSide::Bottom, Self::Center) => class!("[position-area:bottom]"),
            (FloatingSide::Bottom, Self::End) => class!("[position-area:bottom_span-left]"),
            (FloatingSide::Left, Self::Start) => class!("[position-area:left_span-bottom]"),
            (FloatingSide::Left, Self::Center) => class!("[position-area:left]"),
            (FloatingSide::Left, Self::End) => class!("[position-area:left_span-top]"),
        }
    }
}

/// What a floating panel is anchored to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FloatingAnchor {
    /// The button whose `popovertarget` opened the panel. The panel must be a `popover`
    /// element. It shows above the rest of the page, so no container clips it.
    Invoker,
    /// The nearest ancestor with [`FLOATING_SCOPE`]. Use it for panels that open
    /// without the popover API, such as on hover. Containers that clip their overflow do
    /// not cut the panel off, unless they also have a transform, filter, or containment.
    Scope,
}

impl FloatingAnchor {
    /// Classes that anchor the panel. A popover also undoes the browser's centering of
    /// popovers and fades in when it opens.
    fn classes(self) -> StaticClass {
        match self {
            Self::Invoker => class!(
                "inset-auto m-0 [position-anchor:auto]",
                "transition-opacity starting:open:opacity-0 motion-reduce:transition-none",
            ),
            Self::Scope => class!("fixed [position-anchor:--floating-anchor]"),
        }
    }
}

/// Classes that make an element the anchor of the [`FloatingAnchor::Scope`] panels
/// inside it.
///
/// The anchor is scoped to the element, so every instance anchors its own panels.
pub const FLOATING_SCOPE: StaticClass =
    class!("[anchor-name:--floating-anchor] [anchor-scope:--floating-anchor]");

/// Builds the full class list for a panel floating on `side` of its `anchor`, lined up
/// by `align`.
///
/// The panel moves to another position when it would overflow the viewport. Use it to
/// float your own panel next to a button:
///
/// ```ignore
/// let filters = element_id(cx);
/// view! {
///     <button type="button" popovertarget=(&filters)>"Filters"</button>
///     <div
///         id=(&filters)
///         popover="auto"
///         class=(floating_variants(
///             FloatingAnchor::Invoker,
///             FloatingSide::Right,
///             FloatingAlign::Start,
///         ))
///     >
///         (filter_form)
///     </div>
/// }
/// ```
///
/// Or next to an ancestor, for example to show a hint on hover:
///
/// ```ignore
/// view! {
///     <span class=(class!("group inline-flex", FLOATING_SCOPE))>
///         (trigger)
///         <span
///             class=(class!(
///                 floating_variants(
///                     FloatingAnchor::Scope,
///                     FloatingSide::Top,
///                     FloatingAlign::Center,
///                 ),
///                 "invisible group-hover:visible",
///             ))
///         >
///             (hint)
///         </span>
///     </span>
/// }
/// ```
#[must_use]
pub fn floating_variants(
    anchor: FloatingAnchor,
    side: FloatingSide,
    align: FloatingAlign,
) -> Class<(StaticClass, StaticClass, StaticClass)> {
    class!(anchor.classes(), side.classes(), align.area(side))
}
