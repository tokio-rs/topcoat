use topcoat::{
    Result,
    context::Cx,
    view::{AttributeValueViewParts, Child, View, component, view},
};

use super::icons::{self, icon};

/// A breadcrumb trail of [`crumb`] links ending in the current page.
#[component]
pub async fn breadcrumbs(cx: &Cx, current: &str, child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <nav aria-label="Breadcrumb" class="text-sm">
            <ol class="flex flex-wrap items-center gap-1.5 text-stone-500">
                (child)
                <li>
                    <span aria-current="page" class="font-medium text-stone-900">
                        (current)
                    </span>
                </li>
            </ol>
        </nav>
    })
}

/// One link in a breadcrumb trail.
#[component]
pub async fn crumb(
    cx: &Cx,
    href: impl AttributeValueViewParts + Send,
    label: &str,
) -> Result<impl View> {
    Ok(view! {
        <li class="flex items-center gap-1.5">
            <a href=(href) class="hover:text-stone-900">(label)</a>
            icon(icon: icons::CHEVRON_RIGHT)
        </li>
    })
}
