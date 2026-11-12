use maud::{Markup, html};

use super::icons;
use crate::cart::CartCount;

const NAV_LINKS: [(&str, &str); 5] = [
    ("/products", "All products"),
    ("/products?category=outdoor", "Outdoor"),
    ("/products?category=kitchen", "Kitchen"),
    ("/products?category=electronics", "Electronics"),
    ("/about", "About"),
];

/// The announcement bar and site header with navigation, cart badge and mobile menu toggle.
pub fn header(cart: CartCount) -> Markup {
    html! {
        div class="bg-teal-900 px-4 py-2 text-center text-xs font-medium text-teal-50 sm:text-sm" {
            "Free shipping on orders over $75 and free 30-day returns"
        }
        header class="sticky top-0 z-20 border-b border-stone-200 bg-white/95 backdrop-blur" {
            div class="mx-auto flex max-w-7xl flex-wrap items-center gap-x-10 gap-y-3 px-4 py-3 sm:px-6 lg:px-8" {
                a href="/" class="flex items-center gap-2.5 text-lg font-semibold tracking-tight text-stone-900" {
                    (icons::logo())
                    "Harbor Market"
                }
                nav id="site-nav" aria-label="Main" class="order-last hidden w-full md:order-none md:block md:w-auto" {
                    ul class="flex flex-col gap-1 border-t border-stone-100 pt-3 md:flex-row md:gap-8 md:border-0 md:pt-0" {
                        @for (href, label) in NAV_LINKS {
                            li {
                                a href=(href) class="block rounded-md px-2 py-2 text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900 md:p-0 md:hover:bg-transparent" {
                                    (label)
                                }
                            }
                        }
                    }
                }
                div class="ml-auto flex items-center gap-2" {
                    div class="flex items-center gap-2 rounded-full border border-stone-200 bg-white py-1.5 pl-3 pr-1.5 text-sm font-medium text-stone-700" {
                        (icons::bag())
                        span class="hidden sm:inline" { "Cart" }
                        span data-cart-count class="min-w-6 rounded-full bg-teal-700 px-2 py-0.5 text-center text-xs font-semibold text-white" {
                            (cart)
                        }
                    }
                    button type="button" data-menu-toggle aria-controls="site-nav" aria-expanded="false" class="rounded-md p-2 text-stone-600 hover:bg-stone-100 md:hidden" {
                        span class="sr-only" { "Toggle menu" }
                        (icons::menu())
                    }
                }
            }
        }
    }
}
