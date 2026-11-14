use topcoat::{
    Result,
    context::Cx,
    router::href,
    view::{StaticClass, View, class, component, view},
};

use super::icons::{self, icon};
use crate::{
    app::products::{self, ListingLink},
    data::Listing,
};

const STEP: StaticClass = class!(
    "flex items-center gap-2 rounded-full px-3 py-2 text-sm font-medium text-stone-700 hover:bg-stone-100"
);
const STEP_DISABLED: StaticClass =
    class!("flex items-center gap-2 rounded-full px-3 py-2 text-sm font-medium text-stone-300");
const PAGE: StaticClass = class!(
    "flex size-10 items-center justify-center rounded-full text-sm font-medium text-stone-600 hover:bg-stone-100"
);
const PAGE_CURRENT: StaticClass = class!(
    "flex size-10 items-center justify-center rounded-full bg-stone-900 text-sm font-semibold text-white"
);

/// Previous and next links with a link to every page of the listing.
#[component]
pub async fn pagination(cx: &Cx, listing: &Listing<'_>) -> Result<impl View> {
    let link = |page| {
        href!(products::listing).query(ListingLink {
            category: listing.category_slug(),
            sort: listing.sort,
            page,
        })
    };
    Ok(view! {
        <nav
            aria-label="Pagination"
            class="mt-12 flex items-center justify-between gap-4 border-t border-stone-200 pt-6"
        >
            if listing.page > 1 {
                <a href=(link(listing.page - 1)) class=(STEP)>
                    icon(icon: icons::ARROW_LEFT)
                    "Previous"
                </a>
            } else {
                <span aria-disabled="true" class=(STEP_DISABLED)>
                    icon(icon: icons::ARROW_LEFT)
                    "Previous"
                </span>
            }
            <p class="text-sm text-stone-600 sm:hidden">
                "Page "
                (listing.page)
                " of "
                (listing.page_count)
            </p>
            <ul class="hidden items-center gap-1 sm:flex">
                for page in 1..=listing.page_count {
                    <li>
                        if page == listing.page {
                            <a
                                href=(link(page))
                                aria-current="page"
                                class=(PAGE_CURRENT)
                            >
                                (page)
                            </a>
                        } else {
                            <a href=(link(page)) class=(PAGE)>(page)</a>
                        }
                    </li>
                }
            </ul>
            if listing.page < listing.page_count {
                <a href=(link(listing.page + 1)) class=(STEP)>
                    "Next"
                    icon(icon: icons::ARROW_RIGHT)
                </a>
            } else {
                <span aria-disabled="true" class=(STEP_DISABLED)>
                    "Next"
                    icon(icon: icons::ARROW_RIGHT)
                </span>
            }
        </nav>
    })
}
