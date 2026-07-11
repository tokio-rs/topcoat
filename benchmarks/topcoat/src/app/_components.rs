use topcoat::{
    Result,
    view::{class, component, view},
};

use crate::{
    catalog::{Product, Review, Spec, filled_stars, format_rating},
    urls::products_url,
};

/// The heroicons solid star, shared verbatim by every benchmark app.
const STAR_PATH: &str = "M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.539 1.118l-2.8-2.034a1 1 0 00-1.176 0l-2.8 2.034c-.783.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.363-1.118l-2.8-2.034c-.784-.57-.381-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z";

const PAGE_LINK: &str = "rounded-md px-3 py-2 font-medium text-slate-600 hover:bg-slate-100";
const PAGE_DISABLED: &str = "rounded-md px-3 py-2 font-medium text-slate-300";
const PAGE_CURRENT: &str = "rounded-md bg-indigo-600 px-3 py-2 font-semibold text-white";

const FOOTER_COLUMNS: [(&str, [(&str, &str); 4]); 4] = [
    (
        "Shop",
        [
            ("All products", "/products"),
            ("Audio", "/products?category=audio"),
            ("Displays", "/products?category=displays"),
            ("Wearables", "/products?category=wearables"),
        ],
    ),
    (
        "Support",
        [
            ("Contact", "#"),
            ("Shipping", "#"),
            ("Returns", "#"),
            ("Warranty", "#"),
        ],
    ),
    (
        "Company",
        [
            ("About", "#"),
            ("Careers", "#"),
            ("Press", "#"),
            ("Sustainability", "#"),
        ],
    ),
    (
        "Legal",
        [
            ("Privacy", "#"),
            ("Terms", "#"),
            ("Imprint", "#"),
            ("Cookie settings", "#"),
        ],
    ),
];

#[component]
pub async fn site_nav() -> Result {
    view! {
        <header class="border-b border-slate-200 bg-white">
            <nav
                class="mx-auto flex w-full max-w-6xl items-center justify-between px-4 py-4"
            >
                <a href="/" class="text-lg font-bold tracking-tight">
                    "Meridian Supply"
                </a>
                <div class="flex items-center gap-6 text-sm font-medium text-slate-600">
                    <a href="/" class="hover:text-slate-900">"Home"</a>
                    <a href="/products" class="hover:text-slate-900">"Products"</a>
                    <span
                        class="rounded-full bg-indigo-600 px-3 py-1 text-xs font-semibold text-white"
                    >
                        "Cart (3)"
                    </span>
                </div>
            </nav>
        </header>
    }
}

#[component]
pub async fn site_footer() -> Result {
    view! {
        <footer class="border-t border-slate-200 bg-white">
            <div
                class="mx-auto grid w-full max-w-6xl grid-cols-2 gap-8 px-4 py-10 text-sm md:grid-cols-4"
            >
                for (title, links) in FOOTER_COLUMNS {
                    <div>
                        <h3 class="mb-3 font-semibold text-slate-900">(title)</h3>
                        <ul class="space-y-2 text-slate-500">
                            for (label, href) in links {
                                <li>
                                    <a href=(href) class="hover:text-slate-900">(label)</a>
                                </li>
                            }
                        </ul>
                    </div>
                }
            </div>
            <div class="border-t border-slate-100">
                <p class="mx-auto w-full max-w-6xl px-4 py-4 text-xs text-slate-400">
                    "(c) 2026 Meridian Supply. All rights reserved."
                </p>
            </div>
        </footer>
    }
}

#[component]
pub async fn rating_stars(tenths: u32, size: &str) -> Result {
    let filled = filled_stars(tenths);

    view! {
        <div class="flex">
            for index in 0..5u32 {
                <svg
                    viewBox="0 0 20 20"
                    fill="currentColor"
                    aria-hidden="true"
                    class=(class!(
                        size,
                        "text-amber-400" if index < filled else "text-slate-200",
                    ))
                >
                    <path d=(STAR_PATH)></path>
                </svg>
            }
        </div>
    }
}

#[component]
pub async fn product_card(product: &Product) -> Result {
    view! {
        <a
            href=(("/products/", product.id))
            class="group flex flex-col rounded-xl border border-slate-200 bg-white p-4 shadow-sm transition hover:shadow-md"
        >
            <div
                class="mb-4 flex h-32 items-center justify-center rounded-lg bg-slate-100 text-3xl font-bold text-slate-300"
            >
                (product.initials())
            </div>
            <p class="text-xs font-medium uppercase tracking-wide text-slate-400">
                (&product.category)
            </p>
            <h3
                class="mt-1 text-sm font-semibold text-slate-900 group-hover:text-indigo-600"
            >
                (&product.name)
            </h3>
            <div class="mt-2 flex items-center gap-1">
                rating_stars(tenths: product.rating_tenths, size: "h-4 w-4")
                <span class="text-xs text-slate-500">
                    (format_rating(product.rating_tenths))
                    " ("
                    (product.review_count)
                    ")"
                </span>
            </div>
            <p class="mt-3 text-lg font-bold">(product.price())</p>
        </a>
    }
}

#[component]
pub async fn pagination(
    current: usize,
    page_count: usize,
    sort: Option<&str>,
    category: Option<&str>,
) -> Result {
    view! {
        <nav
            aria-label="Pagination"
            class="mt-10 flex flex-wrap items-center justify-center gap-1 text-sm"
        >
            if current > 1 {
                <a href=(products_url(current - 1, sort, category)) class=(PAGE_LINK)>
                    "Previous"
                </a>
            } else {
                <span class=(PAGE_DISABLED)>"Previous"</span>
            }
            for number in 1..=page_count {
                if number == current {
                    <span aria-current="page" class=(PAGE_CURRENT)>(number)</span>
                } else {
                    <a href=(products_url(number, sort, category)) class=(PAGE_LINK)>
                        (number)
                    </a>
                }
            }
            if current < page_count {
                <a href=(products_url(current + 1, sort, category)) class=(PAGE_LINK)>
                    "Next"
                </a>
            } else {
                <span class=(PAGE_DISABLED)>"Next"</span>
            }
        </nav>
    }
}

#[component]
pub async fn breadcrumbs(category: &str, category_slug: &str, name: &str) -> Result {
    view! {
        <nav aria-label="Breadcrumb" class="text-sm text-slate-500">
            <ol class="flex flex-wrap items-center gap-2">
                <li><a href="/" class="hover:text-slate-900">"Home"</a></li>
                <li>"/"</li>
                <li><a href="/products" class="hover:text-slate-900">"Products"</a></li>
                <li>"/"</li>
                <li>
                    <a
                        href=(("/products?category=", category_slug))
                        class="hover:text-slate-900"
                    >
                        (category)
                    </a>
                </li>
                <li>"/"</li>
                <li class="font-medium text-slate-900">(name)</li>
            </ol>
        </nav>
    }
}

#[component]
pub async fn spec_table(specs: &[Spec]) -> Result {
    view! {
        <div class="mt-6 overflow-hidden rounded-xl border border-slate-200 bg-white">
            <table class="w-full text-sm">
                <tbody>
                    for spec in specs {
                        <tr class="border-b border-slate-100 last:border-0">
                            <th
                                scope="row"
                                class="w-1/3 px-4 py-3 text-left font-medium text-slate-500"
                            >
                                (&spec.key)
                            </th>
                            <td class="px-4 py-3 text-slate-900">(&spec.value)</td>
                        </tr>
                    }
                </tbody>
            </table>
        </div>
    }
}

#[component]
pub async fn review_list(reviews: &[Review]) -> Result {
    view! {
        <div class="mt-6 space-y-6">
            for review in reviews {
                <article class="rounded-xl border border-slate-200 bg-white p-6">
                    <div class="flex flex-wrap items-center justify-between gap-2">
                        <p class="font-semibold text-slate-900">(&review.author)</p>
                        <p class="text-xs text-slate-400">(&review.date)</p>
                    </div>
                    <div class="mt-2 flex items-center gap-2">
                        rating_stars(tenths: review.rating_tenths, size: "h-4 w-4")
                        <p class="text-sm font-medium text-slate-700">
                            (&review.title)
                        </p>
                    </div>
                    <p class="mt-3 text-sm text-slate-600">(&review.body)</p>
                </article>
            }
        </div>
    }
}
