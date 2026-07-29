use topcoat::{
    Result,
    view::{Attributes, View, class, component, view},
};

/// The classes for the [`sheet`] overlay: a layer covering the viewport,
/// veiling the page behind it and holding the panel against one edge.
///
/// It is the dialog's overlay without the padding and the centering, since a
/// sheet lies flush against the side it comes from; which side that is, is
/// the panel's business. As with the dialog, only the open state sets a
/// display, so a closed sheet stays hidden by the browser's own rule.
const OVERLAY: &str = "fixed inset-0 z-50 size-full max-h-none max-w-none overflow-hidden \
    bg-background/80 text-foreground backdrop-blur-sm open:flex";

/// A sheet component: a panel that comes in from an edge of the page.
///
/// It is a [`dialog`](super::dialog::dialog) laid against a side rather than
/// centered, for the things a dialog is too small for: filters, a form, a
/// detail view of the row being read. Everything else is the dialog's: its
/// open state is the `open` parameter, so a link or a form that changes the
/// state behind it is what opens and closes the sheet, and dismissing it in
/// the browser alone needs scripting.
///
/// Child nodes are the sheet's content, normally a single [`sheet_content`]
/// panel, whose `side` decides which edge it lies against. Build the inside
/// out of [`dialog_header`](super::dialog::dialog_header),
/// [`dialog_title`](super::dialog::dialog_title) and the rest. The `attrs`
/// are forwarded to the `<dialog>`; a `class` among them is appended to the
/// computed classes.
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
    open: bool,
    /// Extra attributes for the `<dialog>` element.
    #[default]
    mut attrs: Attributes,
    /// The sheet's content.
    #[default]
    child: View,
) -> Result {
    view! {
        <dialog open=(open) class=(class!(OVERLAY, attrs.remove("class"))) (attrs)>
            (child)
        </dialog>
    }
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
    ///
    /// The panel takes the whole of one axis and is pushed against its edge
    /// by an automatic margin on the other, which is what leaves the veil
    /// showing on the remaining side. It is bordered only along the edge that
    /// faces the page, the one edge of it that is not against the viewport.
    fn classes(self) -> &'static str {
        match self {
            Self::Left => "mr-auto h-full w-full max-w-sm border-r",
            Self::Right => "ml-auto h-full w-full max-w-sm border-l",
            Self::Top => "mb-auto max-h-full w-full border-b",
            Self::Bottom => "mt-auto max-h-full w-full border-t",
        }
    }
}

/// The classes shared by every sheet panel, regardless of side.
///
/// The panel is the dialog's, squared off and stretched to its edge: it sets
/// its own background and text color, stacks its sections in a column, and
/// scrolls within itself once there is more in it than the edge it lies
/// against is long.
const CONTENT: &str = "flex flex-col gap-4 overflow-y-auto border-border bg-background p-6 \
    text-foreground shadow-sm";

/// The panel of a [`sheet`], holding its sections.
///
/// The `side` parameter decides which edge it lies against, defaulting to
/// `Right`. The `attrs` (such as `class`) are forwarded to the underlying
/// `<div>`; a `class` among them is appended to the computed classes, so a
/// wider or narrower sheet is a `max-w-*` class among them.
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
    child: View,
) -> Result {
    view! {
        <div class=(class!(CONTENT, side.classes(), attrs.remove("class"))) (attrs)>
            (child)
        </div>
    }
}
