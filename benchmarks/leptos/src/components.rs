use leptos::prelude::*;

use crate::{
    format::{filled_stars, format_price, format_rating, initials, products_url},
    model::{ProductSummary, ReviewData, SpecData},
};

/// The heroicons solid star, shared verbatim by every benchmark app.
const STAR_PATH: &str = "M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.539 1.118l-2.8-2.034a1 1 0 00-1.176 0l-2.8 2.034c-.783.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.363-1.118l-2.8-2.034c-.784-.57-.381-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z";

const PAGE_LINK: &str = "rounded-md px-3 py-2 font-medium text-slate-600 hover:bg-slate-100";
const PAGE_DISABLED: &str = "rounded-md px-3 py-2 font-medium text-slate-300";
const PAGE_CURRENT: &str = "rounded-md bg-indigo-600 px-3 py-2 font-semibold text-white";

const COLUMNS: [(&str, [(&str, &str); 4]); 4] = [
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
pub fn SiteNav() -> impl IntoView {
    view! {
        <header class="border-b border-slate-200 bg-white">
            <nav class="mx-auto flex w-full max-w-6xl items-center justify-between px-4 py-4">
                <a href="/" class="text-lg font-bold tracking-tight">"Meridian Supply"</a>
                <div class="flex items-center gap-6 text-sm font-medium text-slate-600">
                    <a href="/" class="hover:text-slate-900">"Home"</a>
                    <a href="/products" class="hover:text-slate-900">"Products"</a>
                    <span class="rounded-full bg-indigo-600 px-3 py-1 text-xs font-semibold text-white">
                        "Cart (3)"
                    </span>
                </div>
            </nav>
        </header>
    }
}

#[component]
pub fn SiteFooter() -> impl IntoView {
    view! {
        <footer class="border-t border-slate-200 bg-white">
            <div class="mx-auto grid w-full max-w-6xl grid-cols-2 gap-8 px-4 py-10 text-sm md:grid-cols-4">
                {COLUMNS
                    .iter()
                    .map(|(title, links)| {
                        view! {
                            <div>
                                <h3 class="mb-3 font-semibold text-slate-900">{*title}</h3>
                                <ul class="space-y-2 text-slate-500">
                                    {links
                                        .iter()
                                        .map(|(label, href)| {
                                            view! {
                                                <li>
                                                    <a href=*href class="hover:text-slate-900">{*label}</a>
                                                </li>
                                            }
                                        })
                                        .collect_view()}
                                </ul>
                            </div>
                        }
                    })
                    .collect_view()}
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
pub fn RatingStars(tenths: u32, size: &'static str) -> impl IntoView {
    let filled = filled_stars(tenths);

    view! {
        <div class="flex">
            {(0..5u32)
                .map(|index| {
                    let color = if index < filled { "text-amber-400" } else { "text-slate-200" };
                    view! {
                        <svg
                            viewBox="0 0 20 20"
                            fill="currentColor"
                            aria-hidden="true"
                            class=format!("{size} {color}")
                        >
                            <path d=STAR_PATH/>
                        </svg>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
pub fn ProductCard(product: ProductSummary) -> impl IntoView {
    let ProductSummary {
        id,
        name,
        category,
        price_cents,
        rating_tenths,
        review_count,
    } = product;
    let initials = initials(&name);

    view! {
        <a
            href=format!("/products/{id}")
            class="group flex flex-col rounded-xl border border-slate-200 bg-white p-4 shadow-sm transition hover:shadow-md"
        >
            <div class="mb-4 flex h-32 items-center justify-center rounded-lg bg-slate-100 text-3xl font-bold text-slate-300">
                {initials}
            </div>
            <p class="text-xs font-medium uppercase tracking-wide text-slate-400">{category}</p>
            <h3 class="mt-1 text-sm font-semibold text-slate-900 group-hover:text-indigo-600">
                {name}
            </h3>
            <div class="mt-2 flex items-center gap-1">
                <RatingStars tenths=rating_tenths size="h-4 w-4"/>
                <span class="text-xs text-slate-500">
                    {format!("{} ({review_count})", format_rating(rating_tenths))}
                </span>
            </div>
            <p class="mt-3 text-lg font-bold">{format_price(price_cents)}</p>
        </a>
    }
}

#[component]
pub fn PaginationNav(
    current: usize,
    page_count: usize,
    sort: Option<String>,
    category: Option<String>,
) -> impl IntoView {
    view! {
        <nav
            aria-label="Pagination"
            class="mt-10 flex flex-wrap items-center justify-center gap-1 text-sm"
        >
            {if current > 1 {
                view! {
                    <a href=products_url(current - 1, sort.as_deref(), category.as_deref()) class=PAGE_LINK>
                        "Previous"
                    </a>
                }
                    .into_any()
            } else {
                view! { <span class=PAGE_DISABLED>"Previous"</span> }.into_any()
            }}
            {(1..=page_count)
                .map(|number| {
                    if number == current {
                        view! {
                            <span aria-current="page" class=PAGE_CURRENT>{number}</span>
                        }
                            .into_any()
                    } else {
                        view! {
                            <a
                                href=products_url(number, sort.as_deref(), category.as_deref())
                                class=PAGE_LINK
                            >
                                {number}
                            </a>
                        }
                            .into_any()
                    }
                })
                .collect_view()}
            {if current < page_count {
                view! {
                    <a href=products_url(current + 1, sort.as_deref(), category.as_deref()) class=PAGE_LINK>
                        "Next"
                    </a>
                }
                    .into_any()
            } else {
                view! { <span class=PAGE_DISABLED>"Next"</span> }.into_any()
            }}
        </nav>
    }
}

#[component]
pub fn BreadcrumbsNav(category: String, category_slug: String, name: String) -> impl IntoView {
    view! {
        <nav aria-label="Breadcrumb" class="text-sm text-slate-500">
            <ol class="flex flex-wrap items-center gap-2">
                <li>
                    <a href="/" class="hover:text-slate-900">"Home"</a>
                </li>
                <li>"/"</li>
                <li>
                    <a href="/products" class="hover:text-slate-900">"Products"</a>
                </li>
                <li>"/"</li>
                <li>
                    <a href=format!("/products?category={category_slug}") class="hover:text-slate-900">
                        {category}
                    </a>
                </li>
                <li>"/"</li>
                <li class="font-medium text-slate-900">{name}</li>
            </ol>
        </nav>
    }
}

#[component]
pub fn SpecTable(specs: Vec<SpecData>) -> impl IntoView {
    view! {
        <div class="mt-6 overflow-hidden rounded-xl border border-slate-200 bg-white">
            <table class="w-full text-sm">
                <tbody>
                    {specs
                        .into_iter()
                        .map(|spec| {
                            view! {
                                <tr class="border-b border-slate-100 last:border-0">
                                    <th
                                        scope="row"
                                        class="w-1/3 px-4 py-3 text-left font-medium text-slate-500"
                                    >
                                        {spec.key}
                                    </th>
                                    <td class="px-4 py-3 text-slate-900">{spec.value}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
pub fn ReviewList(reviews: Vec<ReviewData>) -> impl IntoView {
    view! {
        <div class="mt-6 space-y-6">
            {reviews
                .into_iter()
                .map(|review| {
                    view! {
                        <article class="rounded-xl border border-slate-200 bg-white p-6">
                            <div class="flex flex-wrap items-center justify-between gap-2">
                                <p class="font-semibold text-slate-900">{review.author}</p>
                                <p class="text-xs text-slate-400">{review.date}</p>
                            </div>
                            <div class="mt-2 flex items-center gap-2">
                                <RatingStars tenths=review.rating_tenths size="h-4 w-4"/>
                                <p class="text-sm font-medium text-slate-700">{review.title}</p>
                            </div>
                            <p class="mt-3 text-sm text-slate-600">{review.body}</p>
                        </article>
                    }
                })
                .collect_view()}
        </div>
    }
}
