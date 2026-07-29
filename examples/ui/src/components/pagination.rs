use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, View, attributes, class, component, view},
};

use super::button::{ButtonSize, ButtonVariant, button_variants};

/// A pagination component: the links stepping through a list too long for one
/// page.
///
/// Every part of it is a link, so which page the reader is on is a matter of
/// the URL and the server decides what a page holds. The trail is a `<nav>`
/// labelled for assistive technology, holding a [`pagination_content`] list of
/// [`pagination_item`]s. The `attrs` (such as `class`) are forwarded to the
/// `<nav>`; a `class` among them is appended to the computed classes.
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
pub async fn pagination(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
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
    }
}

/// The list of steps in a [`pagination`].
///
/// The steps wrap onto another line rather than overflowing when more of them
/// are listed than the container has room for.
#[component]
pub async fn pagination_content(
    #[default] mut attrs: Attributes,
    #[default] child: View,
) -> Result {
    view! {
        <ul
            class=(class!(
                "flex flex-row flex-wrap items-center justify-center gap-1",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </ul>
    }
}

/// One step of a [`pagination_content`], holding a link or an ellipsis.
#[component]
pub async fn pagination_item(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! { <li class=(attrs.remove("class")) (attrs)>(child)</li> }
}

/// A link to one page of the list, usually labelled with its number.
///
/// The link is dressed as a button: an outlined one for the page being read
/// and a ghost one for the rest, which is also what tells the two apart at a
/// glance. The current page carries `aria-current="page"`. Pass the `href`
/// among the `attrs`.
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
    child: View,
) -> Result {
    let variant = if active {
        ButtonVariant::Outline
    } else {
        ButtonVariant::Ghost
    };

    view! {
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
    }
}

/// The classes for the label of [`pagination_previous`] and
/// [`pagination_next`].
///
/// The label is always in the markup, so it is what names the link to
/// assistive technology and a translated one needs nothing set alongside it.
/// It is only shown once the [`pagination`] is wide enough for it, which is
/// measured against the pagination itself rather than the window: the same
/// pagination is roomy on a page of its own and cramped in a card column.
const LABEL: &str = "sr-only @xs:not-sr-only";

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
) -> Result {
    view! {
        <a
            class=(class!(
                button_variants(ButtonVariant::Ghost, ButtonSize::Md),
                attrs.remove("class"),
            ))
            (attrs)
        >
            icon(data: iconify_icon!("feather:chevron-left"))
            <span class=(LABEL)>(label)</span>
        </a>
    }
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
) -> Result {
    view! {
        <a
            class=(class!(
                button_variants(ButtonVariant::Ghost, ButtonSize::Md),
                attrs.remove("class"),
            ))
            (attrs)
        >
            <span class=(LABEL)>(label)</span>
            icon(data: iconify_icon!("feather:chevron-right"))
        </a>
    }
}

/// A stand-in for the page links left out of a long pagination.
///
/// It shows an ellipsis in place of the skipped pages. The glyph itself says
/// nothing to assistive technology, so a phrase standing in for it is read out
/// instead.
#[component]
pub async fn pagination_ellipsis(#[default] mut attrs: Attributes) -> Result {
    view! {
        <span
            class=(class!(
                "flex size-9 items-center justify-center text-muted-foreground",
                attrs.remove("class"),
            ))
            (attrs)
        >
            icon(
                data: iconify_icon!("feather:more-horizontal"),
                attrs: attributes! { class="size-4" }
            )
            <span class="sr-only">"More pages"</span>
        </span>
    }
}
