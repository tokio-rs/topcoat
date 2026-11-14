use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use super::error_page;
use crate::{
    format::Count,
    model::{self, Category, Sort},
    routes::{ListingQuery, Route},
    views::{Breadcrumbs, Crumb, Pagination, ProductCard},
};

const SIDEBAR_LINK: &str = "flex items-center justify-between gap-3 whitespace-nowrap rounded-lg px-3 py-2 text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900";
const SIDEBAR_LINK_CURRENT: &str = "flex items-center justify-between gap-3 whitespace-nowrap rounded-lg bg-teal-50 px-3 py-2 text-sm font-semibold text-teal-900";
const SORT_LINK: &str = "block rounded-full bg-white px-3 py-1.5 text-sm font-medium text-stone-600 ring-1 ring-inset ring-stone-200 hover:bg-stone-100";
const SORT_LINK_CURRENT: &str = "block rounded-full bg-stone-900 px-3 py-1.5 text-sm font-medium text-white ring-1 ring-inset ring-stone-900";

/// The data of one listing page.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct ListingData {
    /// The selected category, or `None` for all products.
    category: Option<Category>,
    sort: Sort,
    page: usize,
    page_count: usize,
    /// Number of products in the listing.
    total: usize,
    /// 1-based positions of the first and last product on this page.
    first: usize,
    last: usize,
    /// Number of products in the whole catalog.
    product_count: usize,
    categories: Vec<Category>,
    products: Vec<model::ProductCard>,
}

/// Loads one page of products. Empty parameters count as absent.
///
/// Fails with 404 for an unknown category or a page past the end, and with 422 for an invalid
/// sort or page number.
#[server]
async fn listing_data(query: ListingQuery) -> ServerFnResult<ListingData> {
    use crate::store::{Listing, Store};

    let store = Store::get();
    let listing = Listing::new(
        store,
        query.category.as_deref(),
        query.sort.as_deref(),
        query.page.as_deref(),
    )
        .map_err(|error| HttpError::new(error.status(), error.message()))?;
    Ok(ListingData {
        category: listing.category.cloned(),
        sort: listing.sort,
        page: listing.page,
        page_count: listing.page_count,
        total: listing.total,
        first: listing.first_position(),
        last: listing.last_position(),
        product_count: store.product_count(),
        categories: store.categories().to_vec(),
        products: listing.products().map(|product| store.card(product)).collect(),
    })
}

/// `GET /products`: one page of products with category sidebar, sort links and pagination.
#[component]
pub fn Listing(query: ListingQuery) -> Element {
    let data = use_server_future(use_reactive!(|query| listing_data(query)))?;
    let data = data.read();
    match &*data {
        Some(Ok(data)) => listing(data),
        Some(Err(error)) => error_page(error),
        None => VNode::empty(),
    }
}

fn listing(data: &ListingData) -> Element {
    let slug = data.category.as_ref().map(|category| category.slug.as_str());
    let (title, description) = match &data.category {
        Some(category) => (category.name.as_str(), category.description.as_str()),
        None => (
            "All products",
            "Everything in the Harbor Market catalog, from tents to turntables.",
        ),
    };
    // The sidebar and sort links are plain `a` elements with URLs from `Route`: the SPEC marks the
    // current sort with `aria-current="true"` and the current category on every page, while
    // `Link` always adds `aria-current="page"` to a link to the current URL.
    let category_link = |slug| Route::listing(slug, data.sort, 1).to_string();
    let sort_link = |sort| Route::listing(slug, sort, 1).to_string();
    let home = || Crumb::new("Home", Route::Home {});
    let (trail, current) = if slug.is_some() {
        let products = Crumb::new("Products", Route::listing(None, Sort::default(), 1));
        (vec![home(), products], title)
    } else {
        (vec![home()], "Products")
    };
    let product_count = Count(data.product_count as u64);
    let total = Count(data.total as u64);

    rsx! {
        document::Title { "{title} | Harbor Market" }
        div { class: "mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8",
            Breadcrumbs { trail, current }
            div { class: "mt-6 flex flex-col gap-4 border-b border-stone-200 pb-8 md:flex-row md:items-end md:justify-between",
                div {
                    h1 { class: "text-3xl font-semibold tracking-tight sm:text-4xl", {title} }
                    p { class: "mt-2 max-w-2xl text-stone-600", {description} }
                }
                p { class: "shrink-0 text-sm text-stone-500",
                    if data.total == 0 {
                        "0 products"
                    } else {
                        "Showing {data.first} to {data.last} of {total} products"
                    }
                }
            }
            div { class: "mt-8 flex flex-col gap-10 lg:flex-row",
                aside { class: "lg:w-60 lg:shrink-0",
                    h2 { class: "px-3 text-sm font-semibold text-stone-900", "Categories" }
                    ul { class: "-mx-1 mt-3 flex gap-1 overflow-x-auto px-1 pb-2 lg:mx-0 lg:block lg:space-y-1 lg:overflow-visible lg:px-0 lg:pb-0",
                        li {
                            if slug.is_none() {
                                a {
                                    href: category_link(None),
                                    aria_current: "page",
                                    class: SIDEBAR_LINK_CURRENT,
                                    "All products"
                                    span { class: "text-xs text-teal-700", "{product_count}" }
                                }
                            } else {
                                a { href: category_link(None), class: SIDEBAR_LINK,
                                    "All products"
                                    span { class: "text-xs text-stone-400", "{product_count}" }
                                }
                            }
                        }
                        for category in &data.categories {
                            li {
                                if slug == Some(category.slug.as_str()) {
                                    a {
                                        href: category_link(Some(&category.slug)),
                                        aria_current: "page",
                                        class: SIDEBAR_LINK_CURRENT,
                                        "{category.name}"
                                        span { class: "text-xs text-teal-700",
                                            "{Count(category.product_count.into())}"
                                        }
                                    }
                                } else {
                                    a {
                                        href: category_link(Some(&category.slug)),
                                        class: SIDEBAR_LINK,
                                        "{category.name}"
                                        span { class: "text-xs text-stone-400",
                                            "{Count(category.product_count.into())}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "min-w-0 flex-1",
                    div { class: "flex flex-wrap items-center gap-3",
                        h2 { class: "text-sm font-medium text-stone-500", "Sort by" }
                        ul { class: "flex flex-wrap gap-2",
                            for sort in Sort::ALL {
                                li {
                                    if sort == data.sort {
                                        a {
                                            href: sort_link(sort),
                                            aria_current: "true",
                                            class: SORT_LINK_CURRENT,
                                            {sort.label()}
                                        }
                                    } else {
                                        a { href: sort_link(sort), class: SORT_LINK, {sort.label()} }
                                    }
                                }
                            }
                        }
                    }
                    if data.total == 0 {
                        p { class: "mt-8 rounded-2xl border border-dashed border-stone-300 p-12 text-center text-stone-500",
                            "No products in this category yet."
                        }
                    } else {
                        div { class: "mt-8 grid grid-cols-2 gap-x-6 gap-y-10 md:grid-cols-3 xl:grid-cols-4",
                            for card in &data.products {
                                ProductCard { key: "{card.id}", card: card.clone() }
                            }
                        }
                    }
                    if data.page_count > 1 {
                        Pagination {
                            category: slug.map(str::to_owned),
                            sort: data.sort,
                            page: data.page,
                            page_count: data.page_count,
                        }
                    }
                }
            }
        }
    }
}
