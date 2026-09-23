use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

use super::button::{ButtonSize, ButtonVariant, button_variants};

/// Navigation links for a list split across pages.
///
/// Provide links with the URLs for each page. The component renders a labelled
/// `<nav>` containing a `pagination_content` list. Attributes are forwarded to
/// the `<nav>`, and classes are appended.
///
/// ```ignore
/// view! {
///     pagination(
///         pagination_content(
///             pagination_item(pagination_previous(attrs: attributes! { href="?page=1" }))
///             pagination_item(
///                 pagination_link(attrs: attributes! { href="?page=1" }, "1")
///             )
///             pagination_item(
///                 pagination_link(active: true, attrs: attributes! { href="?page=2" }, "2")
///             )
///             pagination_item(pagination_ellipsis())
///             pagination_item(pagination_next(attrs: attributes! { href="?page=3" }))
///         )
///     )
/// }
/// ```
#[component]
pub async fn pagination(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <nav
            aria-label="pagination"
            class=(class!(
                "@container mx-auto flex w-full justify-center",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </nav>
    })
}

/// The list of steps in a [`pagination`].
///
/// The steps wrap onto another line rather than overflowing when more of them
/// are listed than the container has room for.
#[component]
pub async fn pagination_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <ul
            class=(class!(
                "flex flex-row flex-wrap items-center justify-center gap-1",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </ul>
    })
}

/// One step of a [`pagination_content`], holding a link or an ellipsis.
#[component]
pub async fn pagination_item(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! { <li class=(attrs.remove("class")) (attrs)>(child)</li> })
}

/// A link to one page, usually labelled with its number.
///
/// Pass its destination through `href`. The current page has distinct styling
/// and `aria-current="page"`.
#[component]
pub async fn pagination_link(
    /// Whether this link points at the page being read.
    #[default]
    active: bool,
    /// Extra attributes for the `<a>` element.
    #[default]
    mut attrs: Attributes,
    /// The link's label.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    let variant = if active {
        ButtonVariant::Outline
    } else {
        ButtonVariant::Ghost
    };

    Ok(view! {
        <a
            aria-current=(active.then_some("page"))
            class=(class!(
                button_variants(variant, ButtonSize::Icon),
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </a>
    })
}

/// The classes for the label of [`pagination_previous`] and
/// [`pagination_next`].
const LABEL: StaticClass = class!("sr-only @xs:not-sr-only");

/// The link to the page before the one being read.
#[component]
pub async fn pagination_previous(
    /// The link's label.
    #[into]
    #[default(String::from("Previous"))]
    label: String,
    /// Extra attributes for the `<a>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <a
            class=(class!(
                button_variants(ButtonVariant::Ghost, ButtonSize::Md),
                attrs.remove("class"),
            ))
            (attrs)
        >
            icon(data: iconify_icon!("lucide:chevron-left"))
            <span class=(LABEL)>(label)</span>
        </a>
    })
}

/// The link to the page after the one being read.
#[component]
pub async fn pagination_next(
    /// The link's label.
    #[into]
    #[default(String::from("Next"))]
    label: String,
    /// Extra attributes for the `<a>` element.
    #[default]
    mut attrs: Attributes,
) -> Result<impl View> {
    Ok(view! {
        <a
            class=(class!(
                button_variants(ButtonVariant::Ghost, ButtonSize::Md),
                attrs.remove("class"),
            ))
            (attrs)
        >
            <span class=(LABEL)>(label)</span>
            icon(data: iconify_icon!("lucide:chevron-right"))
        </a>
    })
}

/// An ellipsis representing omitted page links.
///
/// Includes a text label for assistive technology.
#[component]
pub async fn pagination_ellipsis(#[default] mut attrs: Attributes) -> Result<impl View> {
    Ok(view! {
        <span
            class=(class!(
                "flex size-9 items-center justify-center text-muted-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            icon(
                data: iconify_icon!("lucide:ellipsis"),
                attrs: attributes! { class="size-4" }
            )
            <span class="sr-only">"More pages"</span>
        </span>
    })
}
