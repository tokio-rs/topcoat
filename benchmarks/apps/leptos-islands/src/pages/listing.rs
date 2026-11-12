use std::sync::Arc;

use leptos::{either::Either, prelude::*};
use leptos_router::hooks::use_query_map;

use super::ErrorPage;
use crate::{
    components::{Breadcrumbs, PageTitle, Pagination, ProductCard},
    format::Count,
    model::{Category, ListingParams, PAGE_SIZE, PageError, ProductSummary, Sort},
    store::{Listing, Store},
    urls::ListingUrl,
};

const SIDEBAR_LINK: &str = "flex items-center justify-between gap-3 whitespace-nowrap rounded-lg px-3 py-2 text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900";
const SIDEBAR_LINK_CURRENT: &str = "flex items-center justify-between gap-3 whitespace-nowrap rounded-lg bg-teal-50 px-3 py-2 text-sm font-semibold text-teal-900";
const SORT_LINK: &str = "block rounded-full bg-white px-3 py-1.5 text-sm font-medium text-stone-600 ring-1 ring-inset ring-stone-200 hover:bg-stone-100";
const SORT_LINK_CURRENT: &str = "block rounded-full bg-stone-900 px-3 py-1.5 text-sm font-medium text-white ring-1 ring-inset ring-stone-900";

/// The data of one listing page.
pub struct ListingData {
    categories: Arc<[Category]>,
    /// The selected category, as an index into `categories`.
    category: Option<usize>,
    sort: Sort,
    page: usize,
    page_count: usize,
    total: usize,
    product_count: usize,
    products: Vec<ProductSummary>,
}

impl ListingData {
    /// Loads one page of products with the category sidebar.
    fn load(store: &Store, params: &ListingParams) -> Result<Self, PageError> {
        let listing = Listing::new(store, params)?;
        Ok(Self {
            categories: store.categories().clone(),
            category: listing
                .category
                .and_then(|category| store.category_index(&category.slug)),
            sort: listing.sort,
            page: listing.page,
            page_count: listing.page_count,
            total: listing.total,
            product_count: store.product_count(),
            products: listing
                .products()
                .map(|product| store.summary(product))
                .collect(),
        })
    }

    fn category(&self) -> Option<&Category> {
        self.category.map(|index| &self.categories[index])
    }

    /// 1-based position of the first product on this page, or 0 if the listing is empty.
    fn first_position(&self) -> usize {
        if self.total == 0 {
            0
        } else {
            (self.page - 1) * PAGE_SIZE + 1
        }
    }

    /// 1-based position of the last product on this page.
    fn last_position(&self) -> usize {
        (self.page - 1) * PAGE_SIZE + self.products.len()
    }
}

/// `GET /products`: one page of products with category sidebar, sort links and pagination.
#[component]
pub fn ListingPage() -> impl IntoView {
    let store = expect_context::<Arc<Store>>();
    let params = use_query_map().with_untracked(|query| ListingParams {
        category: query.get("category"),
        sort: query.get("sort"),
        page: query.get("page"),
    });
    match ListingData::load(&store, &params) {
        Ok(data) => Either::Left(view! { <Listing data /> }),
        Err(error) => Either::Right(view! { <ErrorPage error /> }),
    }
}

#[component]
fn Listing(data: ListingData) -> impl IntoView {
    let category = data.category();
    let slug = category.map(|category| category.slug.clone());
    let (title, description) = match category {
        Some(category) => (category.name.clone(), category.description.clone()),
        None => (
            "All products".to_owned(),
            "Everything in the Harbor Market catalog, from tents to turntables.".to_owned(),
        ),
    };
    let breadcrumbs = if slug.is_some() {
        view! {
            <Breadcrumbs
                trail=vec![
                    ("Home".to_owned(), "/".to_owned()),
                    ("Products".to_owned(), "/products".to_owned()),
                ]
                current=title.clone()
            />
        }
    } else {
        view! {
            <Breadcrumbs
                trail=vec![("Home".to_owned(), "/".to_owned())]
                current="Products".to_owned()
            />
        }
    };
    let result_count = if data.total == 0 {
        "0 products".to_owned()
    } else {
        format!(
            "Showing {} to {} of {} products",
            data.first_position(),
            data.last_position(),
            Count(data.total as u64),
        )
    };
    let sort = data.sort;
    let category_link = |slug: Option<&str>| {
        ListingUrl {
            category: slug,
            sort,
            page: 1,
        }
        .to_string()
    };
    let sort_link = |sort| {
        ListingUrl {
            category: slug.as_deref(),
            sort,
            page: 1,
        }
        .to_string()
    };
    let sidebar_link = |href: String, label: String, count: usize, current: bool| {
        view! {
            <li>
                <a
                    href=href
                    aria-current=current.then_some("page")
                    class=if current { SIDEBAR_LINK_CURRENT } else { SIDEBAR_LINK }
                >
                    {label}
                    <span class=if current {
                        "text-xs text-teal-700"
                    } else {
                        "text-xs text-stone-400"
                    }>{Count(count as u64).to_string()}</span>
                </a>
            </li>
        }
    };

    view! {
        <PageTitle title=title.clone() />
        <div class="mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8">
            {breadcrumbs}
            <div class="mt-6 flex flex-col gap-4 border-b border-stone-200 pb-8 md:flex-row md:items-end md:justify-between">
                <div>
                    <h1 class="text-3xl font-semibold tracking-tight sm:text-4xl">{title}</h1>
                    <p class="mt-2 max-w-2xl text-stone-600">{description}</p>
                </div>
                <p class="shrink-0 text-sm text-stone-500">{result_count}</p>
            </div>
            <div class="mt-8 flex flex-col gap-10 lg:flex-row">
                <aside class="lg:w-60 lg:shrink-0">
                    <h2 class="px-3 text-sm font-semibold text-stone-900">"Categories"</h2>
                    <ul class="-mx-1 mt-3 flex gap-1 overflow-x-auto px-1 pb-2 lg:mx-0 lg:block lg:space-y-1 lg:overflow-visible lg:px-0 lg:pb-0">
                        {sidebar_link(
                            category_link(None),
                            "All products".to_owned(),
                            data.product_count,
                            data.category.is_none(),
                        )}
                        {data
                            .categories
                            .iter()
                            .enumerate()
                            .map(|(index, category)| {
                                sidebar_link(
                                    category_link(Some(&category.slug)),
                                    category.name.clone(),
                                    category.product_count,
                                    data.category == Some(index),
                                )
                            })
                            .collect_view()}
                    </ul>
                </aside>
                <div class="min-w-0 flex-1">
                    <div class="flex flex-wrap items-center gap-3">
                        <h2 class="text-sm font-medium text-stone-500">"Sort by"</h2>
                        <ul class="flex flex-wrap gap-2">
                            {Sort::ALL
                                .map(|option| {
                                    let current = option == sort;
                                    view! {
                                        <li>
                                            <a
                                                href=sort_link(option)
                                                aria-current=current.then_some("true")
                                                class=if current { SORT_LINK_CURRENT } else { SORT_LINK }
                                            >
                                                {option.label()}
                                            </a>
                                        </li>
                                    }
                                })}
                        </ul>
                    </div>
                    {if data.total == 0 {
                        Either::Left(
                            view! {
                                <p class="mt-8 rounded-2xl border border-dashed border-stone-300 p-12 text-center text-stone-500">
                                    "No products in this category yet."
                                </p>
                            },
                        )
                    } else {
                        Either::Right(
                            view! {
                                <div class="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 md:grid-cols-3 xl:grid-cols-4">
                                    {data
                                        .products
                                        .into_iter()
                                        .map(|product| view! { <ProductCard product /> })
                                        .collect_view()}
                                </div>
                            },
                        )
                    }}
                    {(data.page_count > 1)
                        .then(|| {
                            view! {
                                <Pagination
                                    category=slug.clone()
                                    sort=sort
                                    page=data.page
                                    page_count=data.page_count
                                />
                            }
                        })}
                </div>
            </div>
        </div>
    }
}
