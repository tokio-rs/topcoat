use topcoat::{
    Result,
    runtime::Expr,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// Classes for the overlay that covers the viewport.
///
/// Set `display` only for the open state so the native closed state remains hidden.
const OVERLAY: StaticClass = class!(
    "fixed inset-0 z-50 size-full max-h-none max-w-none items-start \
     justify-center overflow-y-auto bg-background/80 p-4 text-foreground backdrop-blur-sm \
     open:flex",
);

/// Classes that fade the overlay in and out.
///
/// `allow-discrete` keeps it displayed through the exit transition. `@starting-style`
/// supplies the entry transition's initial opacity.
const FADE: StaticClass = class!(
    "opacity-0 open:opacity-100 starting:open:opacity-0 \
     [transition:opacity_200ms_ease-out,display_200ms_allow-discrete]",
);

/// A panel displayed over the page.
///
/// Pass a boolean to `open` for a fixed state, or a runtime expression to control it in
/// the browser. The overlay blocks clicks on the page behind it. Focus trapping and
/// Escape dismissal require application scripting.
///
/// Pass a `dialog_content` panel as child content. `attrs` are forwarded to the
/// `<dialog>`, with extra classes added to its classes.
/// Give the dialog an accessible name with `aria-label`, or use `aria-labelledby`
/// to reference its title's ID.
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

/// Classes for the dialog panel. Automatic vertical margins center short panels while
/// keeping the top of an oversized panel reachable by scrolling.
const CONTENT: StaticClass = class!(
    "relative my-auto flex w-full max-w-lg flex-col gap-4 rounded-xl \
     border border-border bg-card p-6 text-card-foreground shadow-sm",
);

/// Classes that scale and fade the panel as the dialog opens or closes.
const MOTION: StaticClass = class!(
    "scale-95 opacity-0 in-[[open]]:scale-100 in-[[open]]:opacity-100 \
     starting:in-[[open]]:scale-95 starting:in-[[open]]:opacity-0 \
     [transition:scale_200ms_ease-out,opacity_200ms_ease-out]",
);

/// The content panel inside a dialog.
///
/// Pass the dialog's sections as children. To override its maximum width, use a more
/// specific class such as `[&]:max-w-xl` in `attrs`, or edit the component's classes.
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

/// Text that explains the dialog's purpose or requested action.
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

/// A row of actions aligned to the right of the dialog. Actions wrap when they do not
/// fit.
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
