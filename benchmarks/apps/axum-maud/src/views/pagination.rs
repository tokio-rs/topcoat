use maud::{Markup, html};

use super::icons;
use crate::{data::Listing, urls::ListingUrl};

const STEP: &str = "flex items-center gap-2 rounded-full px-3 py-2 text-sm font-medium text-stone-700 hover:bg-stone-100";
const STEP_DISABLED: &str =
    "flex items-center gap-2 rounded-full px-3 py-2 text-sm font-medium text-stone-300";
const PAGE: &str = "flex size-10 items-center justify-center rounded-full text-sm font-medium text-stone-600 hover:bg-stone-100";
const PAGE_CURRENT: &str = "flex size-10 items-center justify-center rounded-full bg-stone-900 text-sm font-semibold text-white";

/// Previous and next links with a link to every page of the listing.
pub fn pagination(listing: &Listing) -> Markup {
    let url = |page| ListingUrl {
        category: listing.category_slug(),
        sort: listing.sort,
        page,
    };
    html! {
        nav aria-label="Pagination" class="mt-12 flex items-center justify-between gap-4 border-t border-stone-200 pt-6" {
            @if listing.page > 1 {
                a href=(url(listing.page - 1)) class=(STEP) { (icons::arrow_left()) "Previous" }
            } @else {
                span aria-disabled="true" class=(STEP_DISABLED) { (icons::arrow_left()) "Previous" }
            }
            p class="text-sm text-stone-600 sm:hidden" {
                "Page " (listing.page) " of " (listing.page_count)
            }
            ul class="hidden items-center gap-1 sm:flex" {
                @for page in 1..=listing.page_count {
                    li {
                        @if page == listing.page {
                            a href=(url(page)) aria-current="page" class=(PAGE_CURRENT) { (page) }
                        } @else {
                            a href=(url(page)) class=(PAGE) { (page) }
                        }
                    }
                }
            }
            @if listing.page < listing.page_count {
                a href=(url(listing.page + 1)) class=(STEP) { "Next" (icons::arrow_right()) }
            } @else {
                span aria-disabled="true" class=(STEP_DISABLED) { "Next" (icons::arrow_right()) }
            }
        }
    }
}
