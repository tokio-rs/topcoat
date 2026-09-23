use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

use super::button::{ButtonSize, ButtonVariant, button_variants};

/// Links to the pages of a list that is too long for one page.
///
/// Every part of it is a link, so the current page is part of the URL and
/// the server decides what each page shows. The pagination is a `<nav>`
/// labeled for assistive technology. It holds a [`pagination_content`] list
/// of [`pagination_item`]s.
///
/// The `attrs` (such as `class`) are forwarded to the `<nav>`. A `class`
/// among them is appended to the component's classes. The same holds for the
/// other pagination components.
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
/// When the items do not fit in their container, they wrap onto the next
/// line.
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

/// One entry of a [`pagination_content`], holding a link or an ellipsis.
#[component]
pub async fn pagination_item(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! { <li class=(attrs.remove("class")) (attrs)>(child)</li> })
}

/// A link to one page of the list, usually labeled with its number.
///
/// The link looks like a square button: an outline button for the current
/// page and a ghost button for the others. Set `active` for the current page,
/// which also adds `aria-current="page"`. Pass the `href` in `attrs`.
#[component]
pub async fn pagination_link(
    /// Whether this link points at the current page.
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
///
/// The label is always in the markup, so it names the link for assistive
/// technology. It is only visible when the [`pagination`] itself is wide
/// enough. This uses a container query, so the width of the pagination
/// counts, not the width of the window.
const LABEL: StaticClass = class!("sr-only @xs:not-sr-only");

/// A link to the previous page.
///
/// It shows a chevron and `label`, which defaults to `"Previous"`. The label
/// is only visible when the pagination is wide enough. Pass the `href` in
/// `attrs`.
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

/// A link to the next page.
///
/// It shows `label`, which defaults to `"Next"`, and a chevron. The label is
/// only visible when the pagination is wide enough. Pass the `href` in
/// `attrs`.
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

/// An ellipsis that stands for page links left out of a long pagination.
///
/// Assistive technology reads it as "More pages".
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
