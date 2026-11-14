use dioxus::prelude::*;

use super::icons::{Bag, Logo, Menu};
use crate::{cart::CartCount, model::Sort, routes::Route};

const NAV: &str = "order-last hidden w-full md:order-none md:block md:w-auto";
const NAV_OPEN: &str = "order-last w-full md:order-none md:block md:w-auto";

/// The announcement bar and site header with navigation, cart badge and mobile menu toggle.
///
/// Reads the cart count from the [`CartCount`] signal in context.
#[component]
pub fn Header() -> Element {
    let cart = use_context::<Signal<CartCount>>();
    let mut menu_open = use_signal(|| false);
    let nav_links = [
        (Route::listing(None, Sort::default(), 1), "All products"),
        (Route::category("outdoor"), "Outdoor"),
        (Route::category("kitchen"), "Kitchen"),
        (Route::category("electronics"), "Electronics"),
        (Route::About {}, "About"),
    ];

    rsx! {
        div { class: "bg-teal-900 px-4 py-2 text-center text-xs font-medium text-teal-50 sm:text-sm",
            "Free shipping on orders over $75 and free 30-day returns"
        }
        header { class: "sticky top-0 z-20 border-b border-stone-200 bg-white/95 backdrop-blur",
            div { class: "mx-auto flex max-w-7xl flex-wrap items-center gap-x-10 gap-y-3 px-4 py-3 sm:px-6 lg:px-8",
                Link {
                    to: Route::Home {},
                    class: "flex items-center gap-2.5 text-lg font-semibold tracking-tight text-stone-900",
                    Logo {}
                    "Harbor Market"
                }
                nav {
                    id: "site-nav",
                    aria_label: "Main",
                    class: if menu_open() { NAV_OPEN } else { NAV },
                    ul { class: "flex flex-col gap-1 border-t border-stone-100 pt-3 md:flex-row md:gap-8 md:border-0 md:pt-0",
                        for (to , label) in nav_links {
                            li {
                                Link {
                                    to,
                                    class: "block rounded-md px-2 py-2 text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900 md:p-0 md:hover:bg-transparent",
                                    {label}
                                }
                            }
                        }
                    }
                }
                div { class: "ml-auto flex items-center gap-2",
                    div { class: "flex items-center gap-2 rounded-full border border-stone-200 bg-white py-1.5 pl-3 pr-1.5 text-sm font-medium text-stone-700",
                        Bag {}
                        span { class: "hidden sm:inline", "Cart" }
                        span { class: "min-w-6 rounded-full bg-teal-700 px-2 py-0.5 text-center text-xs font-semibold text-white",
                            "{cart}"
                        }
                    }
                    button {
                        r#type: "button",
                        aria_controls: "site-nav",
                        aria_expanded: if menu_open() { "true" } else { "false" },
                        class: "rounded-md p-2 text-stone-600 hover:bg-stone-100 md:hidden",
                        onclick: move |_| menu_open.toggle(),
                        span { class: "sr-only", "Toggle menu" }
                        Menu {}
                    }
                }
            }
        }
    }
}
