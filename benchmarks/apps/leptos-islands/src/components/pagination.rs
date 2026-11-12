use leptos::prelude::*;

use super::icons;
use crate::{model::Sort, urls::ListingUrl};

const STEP: &str = "flex items-center gap-2 rounded-full px-3 py-2 text-sm font-medium text-stone-700 hover:bg-stone-100";
const STEP_DISABLED: &str =
    "flex items-center gap-2 rounded-full px-3 py-2 text-sm font-medium text-stone-300";
const PAGE: &str = "flex size-10 items-center justify-center rounded-full text-sm font-medium text-stone-600 hover:bg-stone-100";
const PAGE_CURRENT: &str = "flex size-10 items-center justify-center rounded-full bg-stone-900 text-sm font-semibold text-white";

/// Previous and next links with a link to every page of a listing.
#[component]
pub fn Pagination(
    category: Option<String>,
    sort: Sort,
    page: usize,
    page_count: usize,
) -> impl IntoView {
    let url = move |page| {
        ListingUrl {
            category: category.as_deref(),
            sort,
            page,
        }
        .to_string()
    };
    view! {
        <nav
            aria-label="Pagination"
            class="mt-12 flex items-center justify-between gap-4 border-t border-stone-200 pt-6"
        >
            {if page > 1 {
                view! {
                    <a href=url(page - 1) class=STEP>
                        {icons::arrow_left()}
                        "Previous"
                    </a>
                }
                    .into_any()
            } else {
                view! {
                    <span aria-disabled="true" class=STEP_DISABLED>
                        {icons::arrow_left()}
                        "Previous"
                    </span>
                }
                    .into_any()
            }}
            <p class="text-sm text-stone-600 sm:hidden">{format!("Page {page} of {page_count}")}</p>
            <ul class="hidden items-center gap-1 sm:flex">
                {(1..=page_count)
                    .map(|number| {
                        let current = number == page;
                        view! {
                            <li>
                                <a
                                    href=url(number)
                                    aria-current=current.then_some("page")
                                    class=if current { PAGE_CURRENT } else { PAGE }
                                >
                                    {number}
                                </a>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
            {if page < page_count {
                view! {
                    <a href=url(page + 1) class=STEP>
                        "Next"
                        {icons::arrow_right()}
                    </a>
                }
                    .into_any()
            } else {
                view! {
                    <span aria-disabled="true" class=STEP_DISABLED>
                        "Next"
                        {icons::arrow_right()}
                    </span>
                }
                    .into_any()
            }}
        </nav>
    }
}
