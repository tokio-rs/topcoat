use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The classes for the [`dialog`] overlay: a layer that covers the viewport,
/// dims the page behind it, and holds the panel.
///
/// By default the browser sizes a `<dialog>` to its content and keeps it
/// smaller than the viewport. Both limits are removed so the element covers
/// the viewport. The overlay is the background color at a lower opacity with
/// a blur, so it dims the page in both color schemes.
///
/// Only the open state sets `display`. While the dialog is closed, the
/// browser's own `display: none` hides it. Setting `display` in all states
/// would override that and keep the closed dialog on the page.
const OVERLAY: StaticClass = class!(
    "fixed inset-0 z-50 size-full max-h-none max-w-none items-start \
     justify-center overflow-y-auto bg-background/80 p-4 text-foreground backdrop-blur-sm \
     open:flex",
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

/// A panel shown over the page for a single task.
///
/// The dialog is a native `<dialog>` that is open while `open` is true. Pass
/// a boolean for a fixed state, or a runtime expression to open and close it
/// in the browser. The overlay covers the page, so the page behind it cannot
/// be clicked. Trapping focus and closing on Escape need extra scripting.
///
/// Child nodes become the dialog's content, usually a single
/// [`dialog_content`] panel. The `attrs` (such as `class` or `id`) are
/// forwarded to the `<dialog>`. A `class` among them is appended to the
/// component's classes. The same holds for the other dialog components.
///
/// ```ignore
/// view! {
///     dialog(
///         open: confirming,
///         dialog_content(
///             dialog_header(
///                 dialog_title("Delete workspace")
///                 dialog_description("This cannot be undone.")
///             )
///             dialog_footer(
///                 // To close the dialog, navigate to a page that
///                 // renders it closed.
///                 <a
///                     href="/workspace"
///                     class=(button_variants(ButtonVariant::Ghost, ButtonSize::Md))
///                 >
///                     "Cancel"
///                 </a>
///                 button(variant: ButtonVariant::Destructive, "Delete")
///             )
///         )
///     )
/// }
/// ```
#[component]
pub async fn dialog(
    /// Whether the dialog shows.
    #[into]
    open: Expr<bool>,
    /// Extra attributes for the `<dialog>` element.
    #[default]
    mut attrs: Attributes,
    /// The dialog's content.
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

/// The classes for the [`dialog_content`] panel: a raised surface styled like
/// a card, with its sections in a column.
///
/// The panel is centered with automatic vertical margins instead of the
/// overlay's alignment. This keeps its top edge reachable when it is taller
/// than the viewport and the overlay scrolls. It sets its own background and
/// text color, so it looks the same on any background. It is positioned, so
/// a control such as a close button can be placed in one of its corners.
const CONTENT: StaticClass = class!(
    "relative my-auto flex w-full max-w-lg flex-col gap-4 rounded-xl \
     border border-border bg-card p-6 text-card-foreground shadow-sm",
);

/// The classes that animate the panel in and out.
///
/// The panel starts slightly smaller and transparent, and grows to full size
/// while the dialog is open. The animation runs in both directions: when the
/// dialog opens, and in reverse when it closes, while the overlay fades out.
/// `@starting-style` gives the panel its starting size when it first
/// appears, because an element that was not rendered before has no previous
/// size.
const MOTION: StaticClass = class!(
    "scale-95 opacity-0 in-[[open]]:scale-100 in-[[open]]:opacity-100 \
     starting:in-[[open]]:scale-95 starting:in-[[open]]:opacity-0 \
     [transition:scale_200ms_ease-out,opacity_200ms_ease-out]",
);

/// The panel of a [`dialog`], holding the dialog's sections.
///
/// A panel stacks a [`dialog_header`], the dialog's body, and a
/// [`dialog_footer`]. Each of them is optional. The panel fills the width of
/// the overlay up to a maximum width. For a wider or narrower dialog, pass a
/// `max-w-*` class in `attrs`.
#[component]
pub async fn dialog_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class=(class!(CONTENT, MOTION, attrs.remove("class"))) (attrs)>
            (child)
        </div>
    })
}

/// The first section of a [`dialog_content`]. It holds a [`dialog_title`]
/// and an optional [`dialog_description`], stacked vertically.
#[component]
pub async fn dialog_header(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class=(class!("flex flex-col gap-1.5", attrs.remove("class"))) (attrs)>
            (child)
        </div>
    })
}

/// The heading of a [`dialog`], rendered as an `<h2>`.
#[component]
pub async fn dialog_title(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <h2
            class=(class!("text-lg leading-none font-semibold", attrs.remove("class")))
            (attrs)
        >
            (child)
        </h2>
    })
}

/// Muted text under a [`dialog_title`] that explains the dialog, rendered as
/// a `<p>`.
#[component]
pub async fn dialog_description(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <p
            class=(class!("text-sm text-muted-foreground", attrs.remove("class")))
            (attrs)
        >
            (child)
        </p>
    })
}

/// The last section of a [`dialog_content`]: a row of actions, aligned to
/// the right edge of the panel.
///
/// Actions that do not fit in one row wrap onto the next line.
#[component]
pub async fn dialog_footer(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "flex flex-wrap items-center justify-end gap-2",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}
