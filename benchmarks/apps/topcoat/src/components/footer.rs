use topcoat::{
    Result,
    context::Cx,
    router::href,
    view::{AttributeValueViewParts, Child, View, component, view},
};

use super::icons::logo;
use crate::app::{
    self, about, admin,
    api::products::list_products,
    products::{self, ListingLink},
};

/// The site footer with link columns.
#[component]
pub async fn footer(cx: &Cx) -> Result<impl View> {
    let category = |slug| href!(products::listing).query(ListingLink::category(slug));
    Ok(view! {
        <footer class="mt-24 bg-stone-900 text-stone-400">
            <div class="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
                <div class="grid gap-12 md:grid-cols-5">
                    <div class="md:col-span-2">
                        <a
                            href=(href!(app::home))
                            class="flex items-center gap-2.5 text-lg font-semibold tracking-tight text-white"
                        >
                            logo()
                            "Harbor Market"
                        </a>
                        <p class="mt-4 max-w-xs text-sm leading-6">
                            "Well-made gear for every harbor day. Shipped from Portland, Oregon since 2016."
                        </p>
                    </div>
                    footer_column(
                        title: "Shop",
                        footer_link(
                            href: href!(products::listing),
                            label: "All products"
                        )
                        footer_link(
                            href: category("outdoor"),
                            label: "Outdoor & Camping"
                        )
                        footer_link(
                            href: category("kitchen"),
                            label: "Kitchen & Dining"
                        )
                        footer_link(href: category("audio"), label: "Audio")
                        footer_link(href: category("toys"), label: "Toys & Games")
                    )
                    footer_column(
                        title: "Help",
                        footer_link(
                            href: href!(about::about).fragment("shipping"),
                            label: "Shipping & returns"
                        )
                        footer_link(
                            href: href!(about::about).fragment("warranty"),
                            label: "Warranty & repairs"
                        )
                        footer_link(
                            href: href!(about::about).fragment("contact"),
                            label: "Contact us"
                        )
                    )
                    footer_column(
                        title: "Company",
                        footer_link(href: href!(about::about), label: "About us")
                        footer_link(
                            href: href!(admin::orders::orders),
                            label: "Order admin"
                        )
                        footer_link(href: href!(list_products), label: "Product API")
                    )
                </div>
                <div
                    class="mt-12 flex flex-col gap-2 border-t border-stone-800 pt-8 text-xs sm:flex-row sm:justify-between"
                >
                    <p>"Copyright 2026 Harbor Market. All rights reserved."</p>
                    <p>"A demo storefront for comparing web frameworks."</p>
                </div>
            </div>
        </footer>
    })
}

/// A titled column of footer links.
#[component]
async fn footer_column(cx: &Cx, title: &str, child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <div>
            <h2 class="text-sm font-semibold text-white">(title)</h2>
            <ul class="mt-4 space-y-3 text-sm">(child)</ul>
        </div>
    })
}

#[component]
async fn footer_link(
    cx: &Cx,
    href: impl AttributeValueViewParts + Send,
    label: &str,
) -> Result<impl View> {
    Ok(view! { <li><a href=(href) class="hover:text-white">(label)</a></li> })
}
