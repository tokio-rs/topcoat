use std::sync::Arc;

use axum::extract::{Query, State, rejection::QueryRejection};
use maud::{Markup, html};

use super::ErrorPage;
use crate::{
    cart::CartCount,
    data::{Listing, ListingParams, Sort, Store},
    format::Count,
    urls::ListingUrl,
    views::{breadcrumbs, layout, pagination, product_card},
};

const SIDEBAR_LINK: &str = "flex items-center justify-between gap-3 whitespace-nowrap rounded-lg px-3 py-2 text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900";
const SIDEBAR_LINK_CURRENT: &str = "flex items-center justify-between gap-3 whitespace-nowrap rounded-lg bg-teal-50 px-3 py-2 text-sm font-semibold text-teal-900";
const SORT_LINK: &str = "block rounded-full bg-white px-3 py-1.5 text-sm font-medium text-stone-600 ring-1 ring-inset ring-stone-200 hover:bg-stone-100";
const SORT_LINK_CURRENT: &str = "block rounded-full bg-stone-900 px-3 py-1.5 text-sm font-medium text-white ring-1 ring-inset ring-stone-900";

/// `GET /products`: one page of products with category sidebar, sort links and pagination.
pub async fn listing_page(
    State(store): State<Arc<Store>>,
    cart: CartCount,
    query: Result<Query<ListingParams>, QueryRejection>,
) -> Result<Markup, ErrorPage> {
    let Ok(Query(params)) = query else {
        return Err(ErrorPage::invalid(cart));
    };
    let listing =
        Listing::new(&store, &params).map_err(|error| ErrorPage::from_listing(error, cart))?;
    let (title, description) = match listing.category {
        Some(category) => (category.name.as_str(), category.description.as_str()),
        None => (
            "All products",
            "Everything in the Harbor Market catalog, from tents to turntables.",
        ),
    };
    let category_link = |slug| ListingUrl {
        category: slug,
        sort: listing.sort,
        page: 1,
    };
    let sort_link = |sort| ListingUrl {
        category: listing.category_slug(),
        sort,
        page: 1,
    };

    let content = html! {
        div class="mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8" {
            @if listing.category.is_some() {
                (breadcrumbs(&[("Home", "/"), ("Products", "/products")], title))
            } @else {
                (breadcrumbs(&[("Home", "/")], "Products"))
            }
            div class="mt-6 flex flex-col gap-4 border-b border-stone-200 pb-8 md:flex-row md:items-end md:justify-between" {
                div {
                    h1 class="text-3xl font-semibold tracking-tight sm:text-4xl" { (title) }
                    p class="mt-2 max-w-2xl text-stone-600" { (description) }
                }
                p class="shrink-0 text-sm text-stone-500" {
                    @if listing.total == 0 {
                        "0 products"
                    } @else {
                        "Showing " (listing.first_position()) " to " (listing.last_position())
                        " of " (Count(listing.total as u64)) " products"
                    }
                }
            }
            div class="mt-8 flex flex-col gap-10 lg:flex-row" {
                aside class="lg:w-60 lg:shrink-0" {
                    h2 class="px-3 text-sm font-semibold text-stone-900" { "Categories" }
                    ul class="-mx-1 mt-3 flex gap-1 overflow-x-auto px-1 pb-2 lg:mx-0 lg:block lg:space-y-1 lg:overflow-visible lg:px-0 lg:pb-0" {
                        li {
                            @if listing.category.is_none() {
                                a href=(category_link(None)) aria-current="page" class=(SIDEBAR_LINK_CURRENT) {
                                    "All products"
                                    span class="text-xs text-teal-700" { (Count(store.product_count() as u64)) }
                                }
                            } @else {
                                a href=(category_link(None)) class=(SIDEBAR_LINK) {
                                    "All products"
                                    span class="text-xs text-stone-400" { (Count(store.product_count() as u64)) }
                                }
                            }
                        }
                        @for category in store.categories() {
                            li {
                                @if listing.category_slug() == Some(category.slug.as_str()) {
                                    a href=(category_link(Some(&category.slug))) aria-current="page" class=(SIDEBAR_LINK_CURRENT) {
                                        (category.name)
                                        span class="text-xs text-teal-700" { (Count(category.product_count as u64)) }
                                    }
                                } @else {
                                    a href=(category_link(Some(&category.slug))) class=(SIDEBAR_LINK) {
                                        (category.name)
                                        span class="text-xs text-stone-400" { (Count(category.product_count as u64)) }
                                    }
                                }
                            }
                        }
                    }
                }
                div class="min-w-0 flex-1" {
                    div class="flex flex-wrap items-center gap-3" {
                        h2 class="text-sm font-medium text-stone-500" { "Sort by" }
                        ul class="flex flex-wrap gap-2" {
                            @for sort in Sort::ALL {
                                li {
                                    @if sort == listing.sort {
                                        a href=(sort_link(sort)) aria-current="true" class=(SORT_LINK_CURRENT) { (sort.label()) }
                                    } @else {
                                        a href=(sort_link(sort)) class=(SORT_LINK) { (sort.label()) }
                                    }
                                }
                            }
                        }
                    }
                    @if listing.total == 0 {
                        p class="mt-8 rounded-2xl border border-dashed border-stone-300 p-12 text-center text-stone-500" {
                            "No products in this category yet."
                        }
                    } @else {
                        div class="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 md:grid-cols-3 xl:grid-cols-4" {
                            @for product in listing.products() {
                                (product_card(&store, product))
                            }
                        }
                    }
                    @if listing.page_count > 1 {
                        (pagination(&listing))
                    }
                }
            }
        }
    };
    Ok(layout(Some(title), cart, content))
}
