use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The classes for the [`sheet`] overlay: a layer covering the viewport,
/// veiling the page behind it and holding the panel against one edge.
const OVERLAY: StaticClass = class!(
    "fixed inset-0 z-50 size-full max-h-none max-w-none overflow-hidden \
     bg-background/80 text-foreground backdrop-blur-sm open:flex",
);

/// The classes fading the veil in and out.
const FADE: StaticClass = class!(
    "opacity-0 open:opacity-100 starting:open:opacity-0 \
     [transition:opacity_200ms_ease-out,display_200ms_allow-discrete]",
);

/// A panel displayed along an edge of the page.
///
/// `open` accepts a boolean or runtime expression. Place the panel inside
/// `sheet_content` and use its `side` prop to choose the edge. Dialog content
/// components can provide headings and actions.
///
/// Attributes are forwarded to the `<dialog>`, and classes are appended.
/// Focus trapping and closing on Escape require application scripting.
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
    /// Along the right edge, full height. This is where a sheet usually comes
    /// from, since it leaves the start of every line on the page in view.
    #[default]
    Right,
    /// Across the top edge, full width.
    Top,
    /// Across the bottom edge, full width.
    Bottom,
}

impl SheetSide {
    /// The Tailwind classes for this side.
    fn classes(self) -> StaticClass {
        match self {
            Self::Left => class!("mr-auto h-full w-full max-w-sm border-r"),
            Self::Right => class!("ml-auto h-full w-full max-w-sm border-l"),
            Self::Top => class!("mb-auto max-h-full w-full border-b"),
            Self::Bottom => class!("mt-auto max-h-full w-full border-t"),
        }
    }

    /// The Tailwind classes sliding the panel in from this side.
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
const CONTENT: StaticClass = class!(
    "flex flex-col gap-4 overflow-y-auto border-border bg-card p-6 \
     text-card-foreground shadow-sm [transition:translate_200ms_ease-out]",
);

/// The content panel of a sheet.
///
/// `side` defaults to `Right`. Attributes are forwarded to the `<div>`, and
/// classes are appended. Use `max-w-*` classes to change its width.
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
