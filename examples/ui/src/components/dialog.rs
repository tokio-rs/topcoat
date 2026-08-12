use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// The classes for the [`dialog`] overlay: a layer covering the viewport,
/// veiling the page behind it and holding the panel.
///
/// The browser sizes a `<dialog>` to its content and caps it below the
/// viewport, so both are cleared for the element to span the viewport. The
/// veil is the background color at reduced opacity over a blur, so the page
/// behind it recedes in both color schemes without a color of its own.
///
/// Only the open state sets a display: while the dialog is closed the
/// browser's own `display: none` hides it, and an unconditional display would
/// override that and leave the dialog on the page.
const OVERLAY: StaticClass = class!(
    "fixed inset-0 z-50 size-full max-h-none max-w-none items-start \
     justify-center overflow-y-auto bg-background/80 p-4 text-foreground backdrop-blur-sm \
     open:flex",
);

/// The classes fading the veil in and out.
///
/// A dialog goes from not being rendered at all to covering the page, which
/// takes two things beyond the fade itself. `display` is named in the
/// transition with `allow-discrete`, which holds the layer on the page for as
/// long as the fade out lasts instead of taking it away at once. And
/// `@starting-style` gives the layer the value to come from: an element that
/// was not rendered a moment ago has no previous style to leave behind, so
/// without it the fade in has nothing to run from.
const FADE: StaticClass = class!(
    "opacity-0 open:opacity-100 starting:open:opacity-0 \
     [transition:opacity_200ms_ease-out,display_200ms_allow-discrete]",
);

/// A dialog component: a panel over the page for a single task.
///
/// The dialog is a native `<dialog>` whose open state is the `open`
/// parameter, so the server decides whether it shows. Both opening and
/// closing it are a link or a form that changes the state behind `open`,
/// which means the dialog survives a reload and can be linked to. Dismissing
/// it in the browser alone needs scripting, as does trapping focus in it or
/// closing it on Escape; the overlay does cover the page, so what is behind
/// it cannot be clicked.
///
/// Child nodes become the dialog's content, normally a single
/// [`dialog_content`] panel. The `attrs` (such as `class` or `id`) are
/// forwarded to the `<dialog>`; a `class` among them is appended to the
/// computed classes.
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
///                 // Closing the dialog is navigating to a page that
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
    open: bool,
    /// Extra attributes for the `<dialog>` element.
    #[default]
    mut attrs: Attributes,
    /// The dialog's content.
    #[default]
    child: View,
) -> Result {
    view! {
        <dialog
            open=(open)
            class=(class!(OVERLAY, FADE, attrs.remove("class")))
            (attrs)
        >
            (child)
        </dialog>
    }
}

/// The classes for the [`dialog_content`] panel: a raised surface styled like
/// a card, stacking its sections in a column.
///
/// The panel is centered by automatic vertical margins rather than by the
/// overlay's alignment, which keeps its top edge reachable once it grows
/// taller than the viewport and the overlay starts scrolling. It sets its own
/// background and text color, so it reads the same on any ancestor, and is
/// positioned, so a control such as a close button can be placed in one of
/// its corners.
const CONTENT: StaticClass = class!(
    "relative my-auto flex w-full max-w-lg flex-col gap-4 rounded-xl \
     border border-border bg-background p-6 text-foreground shadow-sm",
);

/// The classes bringing the panel in behind the veil.
///
/// It comes up a little short of its size and settles into it, which reads as
/// the panel arriving rather than as the page cutting to it. The panel rests
/// at that smaller size and is brought to full while the dialog around it is
/// open, so it plays both ways: in as the dialog opens, and back out as it
/// closes, for as long as the veil's fade holds the dialog on the page.
/// `@starting-style` gives it the size to come from the first time, since a
/// panel that was not rendered a moment ago has no previous size to leave.
const MOTION: StaticClass = class!(
    "scale-95 opacity-0 in-[[open]]:scale-100 in-[[open]]:opacity-100 \
     starting:in-[[open]]:scale-95 starting:in-[[open]]:opacity-0 \
     [transition:scale_200ms_ease-out,opacity_200ms_ease-out]",
);

/// The panel of a [`dialog`], holding the dialog's sections.
///
/// A panel stacks a [`dialog_header`], the dialog's body, and a
/// [`dialog_footer`], any of which can be omitted. It fills the width of the
/// overlay up to a readable maximum, so a wider or narrower dialog is a
/// `max-w-*` class among the `attrs`.
#[component]
pub async fn dialog_content(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <div class=(class!(CONTENT, MOTION, attrs.remove("class"))) (attrs)>
            (child)
        </div>
    }
}

/// The opening section of a [`dialog_content`], stacking a [`dialog_title`]
/// and an optional [`dialog_description`].
#[component]
pub async fn dialog_header(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <div class=(class!("flex flex-col gap-1.5", attrs.remove("class"))) (attrs)>
            (child)
        </div>
    }
}

/// The heading of a [`dialog`], rendered as an `<h2>`.
#[component]
pub async fn dialog_title(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <h2
            class=(class!(
                "text-lg leading-none font-semibold",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </h2>
    }
}

/// The supporting text under a [`dialog_title`], telling the reader what the
/// dialog is asking of them.
#[component]
pub async fn dialog_description(
    #[default] mut attrs: Attributes,
    #[default] child: View,
) -> Result {
    view! {
        <p
            class=(class!("text-sm text-muted-foreground", attrs.remove("class")))
            (attrs)
        >
            (child)
        </p>
    }
}

/// The closing section of a [`dialog_content`], a row of actions ending at
/// the panel's right edge.
///
/// The row wraps, so actions that do not fit the panel's width move onto
/// their own line instead of overflowing it.
#[component]
pub async fn dialog_footer(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <div
            class=(class!(
                "flex flex-wrap items-center justify-end gap-2",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    }
}
