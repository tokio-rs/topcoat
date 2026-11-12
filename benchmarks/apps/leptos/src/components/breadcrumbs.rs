use leptos::prelude::*;

use super::icons;

/// A breadcrumb trail of `(label, href)` links ending in the current page.
#[component]
pub fn Breadcrumbs(trail: Vec<(String, String)>, current: String) -> impl IntoView {
    view! {
        <nav aria-label="Breadcrumb" class="text-sm">
            <ol class="flex flex-wrap items-center gap-1.5 text-stone-500">
                {trail
                    .into_iter()
                    .map(|(label, href)| {
                        view! {
                            <li class="flex items-center gap-1.5">
                                <a href=href class="hover:text-stone-900">
                                    {label}
                                </a>
                                {icons::chevron_right()}
                            </li>
                        }
                    })
                    .collect_view()}
                <li>
                    <span aria-current="page" class="font-medium text-stone-900">
                        {current}
                    </span>
                </li>
            </ol>
        </nav>
    }
}
