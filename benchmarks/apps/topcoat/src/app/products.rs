pub mod product_id;

use serde::Serialize;
use topcoat::{
    Error, Result,
    context::Cx,
    metadata::Metadata,
    router::{error::not_found, href, page, query_params},
    view::{AttributeValueViewParts, StaticClass, View, class, component, view},
};

use crate::{
    app,
    components::{
        breadcrumbs::{breadcrumbs, crumb},
        error_page::{InvalidRequestError, PageError},
        pagination::pagination,
        product_card::product_card,
    },
    data::{Listing, ListingError, ListingParams, Sort, store},
    format::Count,
};

const SIDEBAR_LINK: StaticClass = class!(
    "flex items-center justify-between gap-3 whitespace-nowrap rounded-lg px-3 py-2 text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900"
);
const SIDEBAR_LINK_CURRENT: StaticClass = class!(
    "flex items-center justify-between gap-3 whitespace-nowrap rounded-lg bg-teal-50 px-3 py-2 text-sm font-semibold text-teal-900"
);
const SORT_LINK: StaticClass = class!(
    "block rounded-full bg-white px-3 py-1.5 text-sm font-medium text-stone-600 ring-1 ring-inset ring-stone-200 hover:bg-stone-100"
);
const SORT_LINK_CURRENT: StaticClass = class!(
    "block rounded-full bg-stone-900 px-3 py-1.5 text-sm font-medium text-white ring-1 ring-inset ring-stone-900"
);

/// The query of a link to a listing page. Parameters at their default value are left out.
#[derive(Clone, Copy, Serialize)]
pub struct ListingLink<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<&'a str>,
    #[serde(skip_serializing_if = "Sort::is_default")]
    pub sort: Sort,
    #[serde(skip_serializing_if = "is_first_page")]
    pub page: usize,
}

impl<'a> ListingLink<'a> {
    /// The first page of a category in the default order.
    pub fn category(slug: &'a str) -> Self {
        Self {
            category: Some(slug),
            sort: Sort::default(),
            page: 1,
        }
    }
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes fields by reference"
)]
fn is_first_page(page: &usize) -> bool {
    *page == 1
}

/// Resolves the request's query parameters to a page of products.
fn current_listing(cx: &Cx) -> Result<Listing<'_>> {
    let params = query_params::<ListingParams>(cx).map_err(|_| InvalidRequestError)?;
    Listing::new(store(cx), params).map_err(|error| -> Error {
        match error {
            ListingError::CategoryNotFound | ListingError::PageNotFound => not_found().into(),
            ListingError::InvalidSort | ListingError::InvalidPage => InvalidRequestError.into(),
        }
    })
}

/// `GET /products`: one page of products with category sidebar, sort links and pagination.
#[page(
    with(Metadata::from_fn(
        async |cx| {
            let title = match current_listing(cx) {
                Ok(Listing { category: Some(category), .. }) => format!(
                    "{} | Harbor Market",
                    category.name,
                ),
                Ok(_) => "All products | Harbor Market".to_owned(),
                Err(error) => PageError::of(&error)
                    .map_or("Harbor Market", PageError::document_title)
                    .to_owned(),
            };
            Ok(Metadata::new().title(title))
        },
    ))
)]
pub async fn listing(cx: &Cx) -> Result<impl View> {
    let store = store(cx);
    let current = current_listing(cx)?;
    let (title, description) = match current.category {
        Some(category) => (category.name.as_str(), category.description.as_str()),
        None => (
            "All products",
            "Everything in the Harbor Market catalog, from tents to turntables.",
        ),
    };
    // Category links keep the current sort, and sort links keep the current category.
    let (current_category, current_sort) = (current.category_slug(), current.sort);
    let category_link = move |category| {
        href!(listing).query(ListingLink {
            category,
            sort: current_sort,
            page: 1,
        })
    };
    let sort_link = move |sort| {
        href!(listing).query(ListingLink {
            category: current_category,
            sort,
            page: 1,
        })
    };

    Ok(view! {
        <div class="mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8">
            if current.category.is_some() {
                breadcrumbs(
                    current: title,
                    crumb(href: href!(app::home), label: "Home")
                    crumb(href: href!(listing), label: "Products")
                )
            } else {
                breadcrumbs(
                    current: "Products",
                    crumb(href: href!(app::home), label: "Home")
                )
            }
            <div
                class="mt-6 flex flex-col gap-4 border-b border-stone-200 pb-8 md:flex-row md:items-end md:justify-between"
            >
                <div>
                    <h1 class="text-3xl font-semibold tracking-tight sm:text-4xl">
                        (title)
                    </h1>
                    <p class="mt-2 max-w-2xl text-stone-600">(description)</p>
                </div>
                <p class="shrink-0 text-sm text-stone-500">
                    if current.total == 0 {
                        "0 products"
                    } else {
                        "Showing "
                        (current.first_position())
                        " to "
                        (current.last_position())
                        " of "
                        (Count(current.total as u64).to_string())
                        " products"
                    }
                </p>
            </div>
            <div class="mt-8 flex flex-col gap-10 lg:flex-row">
                <aside class="lg:w-60 lg:shrink-0">
                    <h2 class="px-3 text-sm font-semibold text-stone-900">
                        "Categories"
                    </h2>
                    <ul
                        class="-mx-1 mt-3 flex gap-1 overflow-x-auto px-1 pb-2 lg:mx-0 lg:block lg:space-y-1 lg:overflow-visible lg:px-0 lg:pb-0"
                    >
                        sidebar_link(
                            href: category_link(None),
                            label: "All products",
                            count: store.product_count(),
                            selected: current.category.is_none()
                        )
                        #[key(&category.slug)]
                        for category in store.categories() {
                            sidebar_link(
                                href: category_link(Some(&category.slug)),
                                label: &category.name,
                                count: category.product_count,
                                selected: current.category_slug()
                                    == Some(category.slug.as_str())
                            )
                        }
                    </ul>
                </aside>
                <div class="min-w-0 flex-1">
                    <div class="flex flex-wrap items-center gap-3">
                        <h2 class="text-sm font-medium text-stone-500">"Sort by"</h2>
                        <ul class="flex flex-wrap gap-2">
                            for sort in Sort::ALL {
                                let selected = sort == current.sort;
                                <li>
                                    <a
                                        href=(sort_link(sort))
                                        aria-current=(selected.then_some("true"))
                                        class=(if selected { SORT_LINK_CURRENT } else { SORT_LINK })
                                    >
                                        (sort.label())
                                    </a>
                                </li>
                            }
                        </ul>
                    </div>
                    if current.total == 0 {
                        <p
                            class="mt-8 rounded-2xl border border-dashed border-stone-300 p-12 text-center text-stone-500"
                        >
                            "No products in this category yet."
                        </p>
                    } else {
                        <div
                            class="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 md:grid-cols-3 xl:grid-cols-4"
                        >
                            #[key(product.id)]
                            for product in current.products() {
                                product_card(product: product)
                            }
                        </div>
                    }
                    if current.page_count > 1 {
                        pagination(listing: &current)
                    }
                </div>
            </div>
        </div>
    })
}

/// A category entry in the sidebar, with its product count.
#[component]
async fn sidebar_link(
    cx: &Cx,
    href: impl AttributeValueViewParts + Send,
    label: &str,
    count: usize,
    selected: bool,
) -> Result<impl View> {
    Ok(view! {
        <li>
            <a
                href=(href)
                aria-current=(selected.then_some("page"))
                class=(if selected { SIDEBAR_LINK_CURRENT } else { SIDEBAR_LINK })
            >
                (label)
                <span
                    class=(class!(
                        "text-xs",
                        "text-teal-700" if selected else "text-stone-400",
                    ))
                >
                    (Count(count as u64).to_string())
                </span>
            </a>
        </li>
    })
}
