use topcoat::view::{Class, StaticClass, class};

/// Where a floating panel appears relative to the element it follows.
///
/// The default is [`Bottom`](Self::Bottom).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FloatingSide {
    /// Place the panel above the anchor.
    Top,
    /// Place the panel to the anchor's right.
    Right,
    /// Place the panel below the anchor.
    #[default]
    Bottom,
    /// Place the panel to the anchor's left.
    Left,
}

impl FloatingSide {
    /// Sets the gap from the anchor and the fallback positions. If the panel does not
    /// fit, the browser tries flipping its side, its alignment, then both. Panels on
    /// the left or right can also move above or below the anchor.
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

/// Which edge or center of the anchor a floating panel lines up with.
///
/// The default is [`Center`](Self::Center).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FloatingAlign {
    /// Align the panel with the anchor's left or top edge.
    Start,
    /// Center the panel on the anchor.
    #[default]
    Center,
    /// Align the panel with the anchor's right or bottom edge.
    End,
}

impl FloatingAlign {
    /// Chooses the CSS `position-area` for this side and alignment.
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

/// The element a floating panel follows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FloatingAnchor {
    /// Follow the button that opened the panel through `popovertarget`. The panel
    /// needs a `popover` attribute. It appears above the page, where parent containers
    /// cannot cut it off.
    Invoker,
    /// Follow the nearest parent or ancestor with [`FLOATING_SCOPE`] classes. Use this
    /// for panels shown without the popover API, such as hints shown on hover. Parent
    /// overflow settings do not cut off the panel unless a parent also uses a CSS
    /// transform, filter, or containment.
    Scope,
}

impl FloatingAnchor {
    /// Positions the panel relative to its anchor. For popovers, also removes the
    /// browser's default centering and adds a fade when the panel opens.
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

/// Add these classes to the element that [`FloatingAnchor::Scope`] panels should follow.
///
/// Place the panels inside this element. Each copy of the element positions its own
/// panels independently.
pub const FLOATING_SCOPE: StaticClass =
    class!("[anchor-name:--floating-anchor] [anchor-scope:--floating-anchor]");

/// Returns CSS classes that position a panel next to its anchor.
///
/// `anchor` chooses the element to follow, `side` chooses where the panel appears,
/// and `align` chooses how they line up. The browser tries other positions if the
/// panel does not fit in the window. For a panel opened by a button:
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
/// To show a hint when its parent is hovered, make that parent the anchor:
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
