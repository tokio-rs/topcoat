use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// Classes for a viewport overlay with no edge padding. Set `display` only for the open
/// state so the native closed state remains hidden.
const OVERLAY: StaticClass = class!(
    "fixed inset-0 z-50 size-full max-h-none max-w-none overflow-hidden \
     bg-background/80 text-foreground backdrop-blur-sm open:flex",
);

/// Classes that fade the overlay in and out. `allow-discrete` keeps it displayed
/// through the exit transition, and `@starting-style` supplies the entry opacity.
const FADE: StaticClass = class!(
    "opacity-0 open:opacity-100 starting:open:opacity-0 \
     [transition:opacity_200ms_ease-out,display_200ms_allow-discrete]",
);

/// A panel that slides in from an edge of the page.
///
/// Pass a boolean to `open` for a fixed state, or a runtime expression to control it in
/// the browser. Like [`dialog`](super::dialog::dialog), it needs application scripting
/// for focus trapping and Escape dismissal.
///
/// Pass a `sheet_content` panel as children and choose its edge with `side`. Use dialog
/// components to arrange its contents. `attrs` are forwarded to the `<dialog>`, with
/// extra classes added to its classes.
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
/// [`Default`] is `SheetSide::Right`, used when no side is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SheetSide {
    /// Along the left edge, full height.
    Left,
    /// Along the right edge at full height.
    #[default]
    Right,
    /// Across the top edge, full width.
    Top,
    /// Across the bottom edge, full width.
    Bottom,
}

impl SheetSide {
    /// Classes that position the panel at the selected edge.
    fn classes(self) -> StaticClass {
        match self {
            Self::Left => class!("mr-auto h-full w-full max-w-sm border-r"),
            Self::Right => class!("ml-auto h-full w-full max-w-sm border-l"),
            Self::Top => class!("mb-auto max-h-full w-full border-b"),
            Self::Bottom => class!("mt-auto max-h-full w-full border-t"),
        }
    }

    /// Classes that slide the panel in and out from the selected edge.
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

/// Classes for a sheet panel with vertically stacked content and internal scrolling.
const CONTENT: StaticClass = class!(
    "flex flex-col gap-4 overflow-y-auto border-border bg-card p-6 \
     text-card-foreground shadow-sm [transition:translate_200ms_ease-out]",
);

/// The content panel inside a sheet.
///
/// `side` chooses its edge and defaults to `Right`. `attrs` are forwarded to the
/// `<div>`, with extra classes added to its classes. Use them to adjust the panel's
/// dimensions.
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
