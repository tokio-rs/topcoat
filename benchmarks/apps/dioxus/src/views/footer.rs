use dioxus::prelude::*;

use super::icons::Logo;
use crate::routes::Route;

/// A footer link: a route of the app, or a URL for anchors and the JSON API, which the router does
/// not serve.
enum FooterLink {
    Route(Route),
    Url(&'static str),
}

/// The site footer with link columns.
#[component]
pub fn Footer() -> Element {
    let columns = [
        (
            "Shop",
            vec![
                (FooterLink::Route(Route::listing(None, Default::default(), 1)), "All products"),
                (FooterLink::Route(Route::category("outdoor")), "Outdoor & Camping"),
                (FooterLink::Route(Route::category("kitchen")), "Kitchen & Dining"),
                (FooterLink::Route(Route::category("audio")), "Audio"),
                (FooterLink::Route(Route::category("toys")), "Toys & Games"),
            ],
        ),
        (
            "Help",
            vec![
                (FooterLink::Url("/about#shipping"), "Shipping & returns"),
                (FooterLink::Url("/about#warranty"), "Warranty & repairs"),
                (FooterLink::Url("/about#contact"), "Contact us"),
            ],
        ),
        (
            "Company",
            vec![
                (FooterLink::Route(Route::About {}), "About us"),
                (FooterLink::Route(Route::Orders {}), "Order admin"),
                (FooterLink::Url("/api/products"), "Product API"),
            ],
        ),
    ];
    rsx! {
        footer { class: "mt-24 bg-stone-900 text-stone-400",
            div { class: "mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8",
                div { class: "grid gap-12 md:grid-cols-5",
                    div { class: "md:col-span-2",
                        Link {
                            to: Route::Home {},
                            class: "flex items-center gap-2.5 text-lg font-semibold tracking-tight text-white",
                            Logo {}
                            "Harbor Market"
                        }
                        p { class: "mt-4 max-w-xs text-sm leading-6",
                            "Well-made gear for every harbor day. Shipped from Portland, Oregon since 2016."
                        }
                    }
                    for (title , links) in columns {
                        div {
                            h2 { class: "text-sm font-semibold text-white", {title} }
                            ul { class: "mt-4 space-y-3 text-sm",
                                for (link , label) in links {
                                    li {
                                        match link {
                                            FooterLink::Route(to) => rsx! {
                                                Link { to, class: "hover:text-white", {label} }
                                            },
                                            FooterLink::Url(href) => rsx! {
                                                a { href, class: "hover:text-white", {label} }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "mt-12 flex flex-col gap-2 border-t border-stone-800 pt-8 text-xs sm:flex-row sm:justify-between",
                    p { "Copyright 2026 Harbor Market. All rights reserved." }
                    p { "A demo storefront for comparing web frameworks." }
                }
            }
        }
    }
}
