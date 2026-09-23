use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// The classes for the [`dialog`] overlay: a layer covering the viewport,
/// veiling the page behind it and holding the panel.
const OVERLAY: StaticClass = class!(
    "fixed inset-0 z-50 size-full max-h-none max-w-none items-start \
     justify-center overflow-y-auto bg-background/80 p-4 text-foreground backdrop-blur-sm \
     open:flex",
);

/// The classes fading the veil in and out.
const FADE: StaticClass = class!(
    "opacity-0 open:opacity-100 starting:open:opacity-0 \
     [transition:opacity_200ms_ease-out,display_200ms_allow-discrete]",
);

/// A panel displayed over the page.
///
/// `open` accepts a boolean or a runtime expression. Focus trapping and closing
/// on Escape require application scripting. The overlay blocks pointer access
/// to the page behind it.
///
/// Place the panel inside `dialog_content`. Attributes are forwarded to the
/// `<dialog>`, and classes are appended.
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
/// a card, stacking its sections in a column.
const CONTENT: StaticClass = class!(
    "relative my-auto flex w-full max-w-lg flex-col gap-4 rounded-xl \
     border border-border bg-card p-6 text-card-foreground shadow-sm",
);

/// The classes bringing the panel in behind the veil.
const MOTION: StaticClass = class!(
    "scale-95 opacity-0 in-[[open]]:scale-100 in-[[open]]:opacity-100 \
     starting:in-[[open]]:scale-95 starting:in-[[open]]:opacity-0 \
     [transition:scale_200ms_ease-out,opacity_200ms_ease-out]",
);

/// A dialog's content panel.
///
/// Arrange its header, body, and footer as needed. Use a `max-w-*` class in
/// `attrs` to change the maximum width.
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

/// The opening section of a [`dialog_content`], stacking a [`dialog_title`]
/// and an optional [`dialog_description`].
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

/// The supporting text under a [`dialog_title`], telling the reader what the
/// dialog is asking of them.
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

/// A row of dialog actions aligned to the right.
///
/// Actions wrap when they do not fit on one line.
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
