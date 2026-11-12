use topcoat::{
    Result,
    context::Cx,
    router::href,
    runtime::{Signal, signal},
    view::{AttributeValueViewParts, View, component, view},
};

use super::icons::{self, icon, logo};
use crate::app::{
    self, about,
    products::{self, ListingLink},
};

/// The announcement bar and site header with navigation, cart badge and mobile menu toggle.
#[component]
pub async fn header(cx: &Cx, cart: &Signal<u32>) -> Result<impl View> {
    let menu_open = signal(cx, || false);
    Ok(view! {
        <div
            class="bg-teal-900 px-4 py-2 text-center text-xs font-medium text-teal-50 sm:text-sm"
        >
            "Free shipping on orders over $75 and free 30-day returns"
        </div>
        <header
            class="sticky top-0 z-20 border-b border-stone-200 bg-white/95 backdrop-blur"
        >
            <div
                class="mx-auto flex max-w-7xl flex-wrap items-center gap-x-10 gap-y-3 px-4 py-3 sm:px-6 lg:px-8"
            >
                <a
                    href=(href!(app::home))
                    class="flex items-center gap-2.5 text-lg font-semibold tracking-tight text-stone-900"
                >
                    logo()
                    "Harbor Market"
                </a>
                <nav
                    id="site-nav"
                    aria-label="Main"
                    :class=$(if menu_open.get() {
                        "order-last w-full md:order-none md:block md:w-auto"
                    } else {
                        "order-last hidden w-full md:order-none md:block md:w-auto"
                    })
                >
                    <ul
                        class="flex flex-col gap-1 border-t border-stone-100 pt-3 md:flex-row md:gap-8 md:border-0 md:pt-0"
                    >
                        nav_link(href: href!(products::listing), label: "All products")
                        nav_link(
                            href: href!(products::listing).query(
                                ListingLink::category("outdoor"),
                            ),
                            label: "Outdoor"
                        )
                        nav_link(
                            href: href!(products::listing).query(
                                ListingLink::category("kitchen"),
                            ),
                            label: "Kitchen"
                        )
                        nav_link(
                            href: href!(products::listing).query(
                                ListingLink::category("electronics"),
                            ),
                            label: "Electronics"
                        )
                        nav_link(href: href!(about::about), label: "About")
                    </ul>
                </nav>
                <div class="ml-auto flex items-center gap-2">
                    <div
                        class="flex items-center gap-2 rounded-full border border-stone-200 bg-white py-1.5 pl-3 pr-1.5 text-sm font-medium text-stone-700"
                    >
                        icon(icon: icons::BAG)
                        <span class="hidden sm:inline">"Cart"</span>
                        <span
                            class="min-w-6 rounded-full bg-teal-700 px-2 py-0.5 text-center text-xs font-semibold text-white"
                        >
                            // Expressions cannot turn numbers into text, so the badge renders either
                            // the capped label or the count.
                            $({
                                let count = cart.get();
                                if count > 99u32 {
                                    (Some("99+"), None)
                                } else {
                                    (None, Some(count))
                                }
                            })
                        </span>
                    </div>
                    <button
                        type="button"
                        aria-controls="site-nav"
                        :aria-expanded=$(if menu_open.get() { "true" } else { "false" })
                        @click=$(|_e| menu_open.toggle())
                        class="rounded-md p-2 text-stone-600 hover:bg-stone-100 md:hidden"
                    >
                        <span class="sr-only">"Toggle menu"</span>
                        icon(icon: icons::MENU)
                    </button>
                </div>
            </div>
        </header>
    })
}

/// One link of the main navigation.
#[component]
async fn nav_link(
    cx: &Cx,
    href: impl AttributeValueViewParts + Send,
    label: &str,
) -> Result<impl View> {
    Ok(view! {
        <li>
            <a
                href=(href)
                class="block rounded-md px-2 py-2 text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900 md:p-0 md:hover:bg-transparent"
            >
                (label)
            </a>
        </li>
    })
}
