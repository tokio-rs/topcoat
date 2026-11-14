use leptos::prelude::*;

use super::icons;

const COLUMNS: [(&str, &[(&str, &str)]); 3] = [
    (
        "Shop",
        &[
            ("/products", "All products"),
            ("/products?category=outdoor", "Outdoor & Camping"),
            ("/products?category=kitchen", "Kitchen & Dining"),
            ("/products?category=audio", "Audio"),
            ("/products?category=toys", "Toys & Games"),
        ],
    ),
    (
        "Help",
        &[
            ("/about#shipping", "Shipping & returns"),
            ("/about#warranty", "Warranty & repairs"),
            ("/about#contact", "Contact us"),
        ],
    ),
    (
        "Company",
        &[
            ("/about", "About us"),
            ("/admin/orders", "Order admin"),
            ("/api/products", "Product API"),
        ],
    ),
];

/// The site footer with link columns.
#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="mt-24 bg-stone-900 text-stone-400">
            <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
                <div class="grid gap-12 md:grid-cols-5">
                    <div class="md:col-span-2">
                        <a
                            href="/"
                            class="flex items-center gap-2.5 text-lg font-semibold tracking-tight text-white"
                        >
                            {icons::logo()}
                            "Harbor Market"
                        </a>
                        <p class="mt-4 max-w-xs text-sm leading-6">
                            "Well-made gear for every harbor day. Shipped from Portland, Oregon since 2016."
                        </p>
                    </div>
                    {COLUMNS
                        .map(|(title, links)| {
                            view! {
                                <div>
                                    <h2 class="text-sm font-semibold text-white">{title}</h2>
                                    <ul class="mt-4 space-y-3 text-sm">
                                        {links
                                            .iter()
                                            .map(|&(href, label)| {
                                                // The JSON API is not a page of the app, so the router must not handle it.
                                                let external = href.starts_with("/api/").then_some("external");
                                                view! {
                                                    <li>
                                                        <a href=href rel=external class="hover:text-white">
                                                            {label}
                                                        </a>
                                                    </li>
                                                }
                                            })
                                            .collect_view()}
                                    </ul>
                                </div>
                            }
                        })}
                </div>
                <div class="mt-12 flex flex-col gap-2 border-t border-stone-800 pt-8 text-xs sm:flex-row sm:justify-between">
                    <p>"Copyright 2026 Harbor Market. All rights reserved."</p>
                    <p>"A demo storefront for comparing web frameworks."</p>
                </div>
            </div>
        </footer>
    }
}
