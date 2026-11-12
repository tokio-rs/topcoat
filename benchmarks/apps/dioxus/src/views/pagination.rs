use dioxus::prelude::*;

use super::icons::{ArrowLeft, ArrowRight};
use crate::{model::Sort, routes::Route};

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
) -> Element {
    let url = |page| Route::listing(category.as_deref(), sort, page);
    rsx! {
        nav {
            aria_label: "Pagination",
            class: "mt-12 flex items-center justify-between gap-4 border-t border-stone-200 pt-6",
            if page > 1 {
                Link { to: url(page - 1), class: STEP,
                    ArrowLeft {}
                    "Previous"
                }
            } else {
                span { aria_disabled: "true", class: STEP_DISABLED,
                    ArrowLeft {}
                    "Previous"
                }
            }
            p { class: "text-sm text-stone-600 sm:hidden", "Page {page} of {page_count}" }
            ul { class: "hidden items-center gap-1 sm:flex",
                for number in 1..=page_count {
                    li {
                        if number == page {
                            // `Link` marks the link to the current page with `aria-current="page"`.
                            Link { to: url(number), class: PAGE_CURRENT, "{number}" }
                        } else {
                            Link { to: url(number), class: PAGE, "{number}" }
                        }
                    }
                }
            }
            if page < page_count {
                Link { to: url(page + 1), class: STEP,
                    "Next"
                    ArrowRight {}
                }
            } else {
                span { aria_disabled: "true", class: STEP_DISABLED,
                    "Next"
                    ArrowRight {}
                }
            }
        }
    }
}
