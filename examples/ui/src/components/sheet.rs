use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The classes for the [`sheet`] overlay: a layer that covers the viewport,
/// dims the page behind it, and holds the panel against one edge.
///
/// It is the dialog's overlay without padding and centering, because a sheet
/// touches the edge it comes from. The panel decides which edge that is. As
/// with the dialog, only the open state sets `display`, so the browser's own
/// rule hides a closed sheet.
const OVERLAY: StaticClass = class!(
    "fixed inset-0 z-50 size-full max-h-none max-w-none overflow-hidden \
     bg-background/80 text-foreground backdrop-blur-sm open:flex",
);

/// The classes that fade the overlay in and out.
///
/// The transition lists `display` with `allow-discrete`, which keeps the
/// overlay on the page until the fade out ends. `@starting-style` gives the
/// fade in its starting opacity. An element that was not rendered before has
/// no previous style, so without it there would be nothing to fade from.
const FADE: StaticClass = class!(
    "opacity-0 open:opacity-100 starting:open:opacity-0 \
     [transition:opacity_200ms_ease-out,display_200ms_allow-discrete]",
);

/// A panel that slides in from an edge of the page.
///
/// It works like a [`dialog`](super::dialog::dialog), but the panel sits
/// against an edge instead of in the center. Use it for content that is too
/// large for a dialog, such as filters, a form, or a detail view. The sheet
/// is open while `open` is true. Pass a boolean for a fixed state, or a
/// runtime expression to open and close it in the browser.
///
/// Child nodes become the sheet's content, usually a single
/// [`sheet_content`] panel. Its `side` decides the edge. Build the panel's
/// content from [`dialog_header`](super::dialog::dialog_header),
/// [`dialog_title`](super::dialog::dialog_title), and the other dialog
/// components. The `attrs` are forwarded to the `<dialog>`. A `class` among
/// them is appended to the component's classes.
///
/// ```ignore
/// view! {
///     sheet(
///         open: filtering,
///         sheet_content(
///             dialog_header(
///                 dialog_title("Filters")
///                 dialog_description("Narrow the deployments below.")
///             )
///             (fields)
///         )
///     )
/// }
/// ```
#[component]
pub async fn sheet(
    /// Whether the sheet shows.
    #[into]
    open: Expr<bool>,
    /// Extra attributes for the `<dialog>` element.
    #[default]
    mut attrs: Attributes,
    /// The sheet's content.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <dialog
            :open=(open)
            class=(class!(OVERLAY, FADE, attrs.remove("class")))
            (attrs)
        >
            (child)
        </dialog>
    })
}

/// The edge a [`sheet_content`] lies against.
///
/// The default is `SheetSide::Right`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SheetSide {
    /// Along the left edge, full height.
    Left,
    /// Along the right edge, full height. This is the usual side, because it
    /// keeps the start of each line on the page visible.
    #[default]
    Right,
    /// Across the top edge, full width.
    Top,
    /// Across the bottom edge, full width.
    Bottom,
}

impl SheetSide {
    /// The Tailwind classes for this side.
    ///
    /// The panel fills one axis, and an automatic margin pushes it against its
    /// edge on the other axis. The rest of the overlay stays visible. The
    /// panel only has a border on the side that faces the page.
    fn classes(self) -> StaticClass {
        match self {
            Self::Left => class!("mr-auto h-full w-full max-w-sm border-r"),
            Self::Right => class!("ml-auto h-full w-full max-w-sm border-l"),
            Self::Top => class!("mb-auto max-h-full w-full border-b"),
            Self::Bottom => class!("mt-auto max-h-full w-full border-t"),
        }
    }

    /// The Tailwind classes that slide the panel in from this side.
    ///
    /// The panel starts outside the edge and moves in while the sheet is
    /// open. The animation runs in both directions: in when the sheet opens,
    /// and out when it closes, while the overlay fades out.
    /// `@starting-style` gives the panel its starting position when it first
    /// appears, because an element that was not rendered before has no
    /// previous position.
    fn motion(self) -> StaticClass {
        match self {
            Self::Left => class!(
                "-translate-x-full in-[[open]]:translate-x-0 \
                 starting:in-[[open]]:-translate-x-full",
            ),
            Self::Right => class!(
                "translate-x-full in-[[open]]:translate-x-0 \
                 starting:in-[[open]]:translate-x-full",
            ),
            Self::Top => class!(
                "-translate-y-full in-[[open]]:translate-y-0 \
                 starting:in-[[open]]:-translate-y-full",
            ),
            Self::Bottom => class!(
                "translate-y-full in-[[open]]:translate-y-0 \
                 starting:in-[[open]]:translate-y-full",
            ),
        }
    }
}

/// The classes shared by every sheet panel, regardless of side.
///
/// The panel is like the dialog's panel, but without rounded corners and
/// stretched along its edge. It sets its own background and text color,
/// stacks its sections in a column, and scrolls when its content is longer
/// than the edge.
const CONTENT: StaticClass = class!(
    "flex flex-col gap-4 overflow-y-auto border-border bg-card p-6 \
     text-card-foreground shadow-sm [transition:translate_200ms_ease-out]",
);

/// The panel of a [`sheet`], holding its sections.
///
/// `side` sets the edge the panel sits against and defaults to `Right`. The
/// `attrs` (such as `class`) are forwarded to the `<div>`. A `class` among
/// them is appended to the component's classes. For a wider or narrower
/// left or right sheet, pass a `max-w-*` class.
#[component]
pub async fn sheet_content(
    /// The edge the panel lies against.
    #[default]
    side: SheetSide,
    /// Extra attributes for the `<div>` element.
    #[default]
    mut attrs: Attributes,
    /// The panel's sections.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(CONTENT, side.classes(), side.motion(), attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}
