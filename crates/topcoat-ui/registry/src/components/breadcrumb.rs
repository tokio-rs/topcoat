use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Child, View, attributes, class, component, view},
};

/// The path from the root of the site to the current page.
///
/// The breadcrumb is a `<nav>` labeled as a breadcrumb. It holds a
/// [`breadcrumb_list`] of [`breadcrumb_item`]s, with a
/// [`breadcrumb_separator`] between them. Each item holds a
/// [`breadcrumb_link`], except the last one, which holds a
/// [`breadcrumb_page`] for the current page.
///
/// The `attrs` (such as `class`) are forwarded to the `<nav>`. The other
/// breadcrumb components also forward their `attrs` to their element and
/// append a `class` among them to their own classes.
///
/// ```ignore
/// view! {
///     breadcrumb(
///         breadcrumb_list(
///             breadcrumb_item(
///                 breadcrumb_link(attrs: attributes! { href="/" }, "Home")
///             )
///             breadcrumb_separator()
///             breadcrumb_item(breadcrumb_page("Settings"))
///         )
///     )
/// }
/// ```
#[component]
pub async fn breadcrumb(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <nav aria-label="breadcrumb" class=(attrs.remove("class")) (attrs)>(child)</nav>
    })
}

/// The ordered list of steps in a [`breadcrumb`].
///
/// When the steps do not fit in their container, they wrap onto the next
/// line.
#[component]
pub async fn breadcrumb_list(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <ol
            class=(class!(
                "flex flex-wrap items-center gap-2 text-sm text-muted-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </ol>
    })
}

/// One step of a [`breadcrumb_list`], holding a [`breadcrumb_link`] or a
/// [`breadcrumb_page`].
#[component]
pub async fn breadcrumb_item(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <li
            class=(class!("inline-flex items-center gap-2", attrs.remove("class")))
            (attrs)
        >
            (child)
        </li>
    })
}

/// A link to a page above the current one, rendered as an `<a>`.
///
/// It uses the muted color of the list, and the foreground color on hover.
/// Pass the `href` in `attrs`.
#[component]
pub async fn breadcrumb_link(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <a
            class=(class!(
                "transition-colors hover:text-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </a>
    })
}

/// The current page, shown as the last step without a link.
///
/// It has `aria-current="page"`, so assistive technology announces it as the
/// current page.
#[component]
pub async fn breadcrumb_page(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <span
            aria-current="page"
            class=(class!("font-medium text-foreground", attrs.remove("class")))
            (attrs)
        >
            (child)
        </span>
    })
}

/// The divider between two steps of a [`breadcrumb_list`].
///
/// It is a chevron in its own `<li>`, hidden from assistive technology.
#[component]
pub async fn breadcrumb_separator(#[default] mut attrs: Attributes) -> Result<impl View> {
    Ok(view! {
        <li aria-hidden="true" class=(attrs.remove("class")) (attrs)>
            icon(
                data: iconify_icon!("lucide:chevron-right"),
                attrs: attributes! { class="size-3.5" }
            )
        </li>
    })
}

/// An ellipsis that stands for steps left out of a long path.
///
/// Assistive technology reads it as "More".
#[component]
pub async fn breadcrumb_ellipsis(#[default] mut attrs: Attributes) -> Result<impl View> {
    Ok(view! {
        <span class=(class!("flex items-center", attrs.remove("class"))) (attrs)>
            icon(
                data: iconify_icon!("lucide:ellipsis"),
                attrs: attributes! { class="size-4" }
            )
            <span class="sr-only">"More"</span>
        </span>
    })
}
