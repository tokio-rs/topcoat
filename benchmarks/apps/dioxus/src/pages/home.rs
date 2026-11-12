use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use super::error_page;
use crate::{
    format::Count,
    model::{self, Category, Sort},
    routes::Route,
    views::{
        ProductCard,
        icons::{ArrowRight, Refresh, Shield, Truck},
    },
};

/// The data of the home page.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct HomeData {
    categories: Vec<Category>,
    featured: Vec<model::ProductCard>,
}

/// Loads the categories with their product counts and the featured products.
#[server]
async fn home_data() -> ServerFnResult<HomeData> {
    let store = crate::store::Store::get();
    Ok(HomeData {
        categories: store.categories().to_vec(),
        featured: store.featured().map(|product| store.card(product)).collect(),
    })
}

/// `GET /`: hero, store perks, category links and featured products.
#[component]
pub fn Home() -> Element {
    let data = use_server_future(home_data)?;
    let data = data.read();
    match &*data {
        Some(Ok(data)) => home(data),
        Some(Err(error)) => error_page(error),
        None => VNode::empty(),
    }
}

fn home(data: &HomeData) -> Element {
    rsx! {
        document::Title { "Harbor Market" }
        section { class: "relative overflow-hidden bg-teal-900 text-white",
            div { class: "mx-auto grid max-w-7xl items-center gap-12 px-4 py-20 sm:px-6 lg:grid-cols-2 lg:px-8 lg:py-28",
                div {
                    p { class: "text-sm font-semibold uppercase tracking-widest text-teal-200",
                        "New season, same harbor"
                    }
                    h1 { class: "mt-4 text-4xl font-semibold tracking-tight text-balance sm:text-5xl lg:text-6xl",
                        "Well-made gear for every harbor day."
                    }
                    p { class: "mt-6 max-w-xl text-lg leading-8 text-teal-100",
                        "Tents, knives, boots and blankets from small workshops we know by name. Built to last, easy to repair and shipped free over $75."
                    }
                    div { class: "mt-10 flex flex-wrap gap-4",
                        Link {
                            to: Route::listing(None, Sort::default(), 1),
                            class: "rounded-full bg-white px-6 py-3 text-sm font-semibold text-teal-900 shadow-sm hover:bg-teal-50",
                            "Shop all products"
                        }
                        Link {
                            to: Route::category("outdoor"),
                            class: "rounded-full px-6 py-3 text-sm font-semibold text-white ring-1 ring-inset ring-white/40 hover:bg-white/10",
                            "Browse outdoor gear"
                        }
                    }
                }
                div {
                    aria_hidden: "true",
                    class: "relative hidden aspect-[4/3] lg:block",
                    div { class: "absolute left-[8%] top-[10%] aspect-square w-[46%] -rotate-6 rounded-[28%] bg-amber-200 shadow-2xl shadow-black/30" }
                    div { class: "absolute right-[6%] top-[4%] aspect-square w-[34%] rotate-12 rounded-full bg-sky-400 shadow-2xl shadow-black/30" }
                    div { class: "absolute bottom-[2%] right-[18%] aspect-square w-[40%] rotate-6 rounded-[28%] bg-orange-700 shadow-2xl shadow-black/30" }
                }
            }
        }
        section {
            aria_label: "Why Harbor Market",
            class: "border-b border-stone-200 bg-white",
            ul { class: "mx-auto grid max-w-7xl gap-8 px-4 py-10 sm:grid-cols-3 sm:px-6 lg:px-8",
                Perk {
                    title: "Free shipping over $75",
                    text: "Orders ship within one business day from Portland.",
                    Truck {}
                }
                Perk {
                    title: "30-day returns",
                    text: "Changed your mind? Send it back, on us.",
                    Refresh {}
                }
                Perk {
                    title: "Repair, don't replace",
                    text: "Spare parts and repairs for as long as we sell it.",
                    Shield {}
                }
            }
        }
        section {
            aria_labelledby: "categories-heading",
            class: "mx-auto max-w-7xl px-4 pt-16 sm:px-6 lg:px-8",
            h2 {
                id: "categories-heading",
                class: "text-2xl font-semibold tracking-tight",
                "Shop by category"
            }
            ul { class: "mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4",
                for category in &data.categories {
                    li {
                        Link {
                            to: Route::category(&category.slug),
                            class: "group block h-full rounded-2xl border border-stone-200 bg-white p-5 transition hover:border-teal-600 hover:shadow-md",
                            h3 { class: "font-semibold text-stone-900 group-hover:text-teal-800",
                                "{category.name}"
                            }
                            p { class: "mt-1 line-clamp-2 text-sm text-stone-600",
                                "{category.description}"
                            }
                            p { class: "mt-4 text-xs font-medium text-stone-500",
                                "{Count(category.product_count.into())} products"
                            }
                        }
                    }
                }
            }
        }
        section {
            aria_labelledby: "featured-heading",
            class: "mx-auto max-w-7xl px-4 pt-20 sm:px-6 lg:px-8",
            div { class: "flex items-end justify-between gap-4",
                h2 {
                    id: "featured-heading",
                    class: "text-2xl font-semibold tracking-tight",
                    "Featured products"
                }
                Link {
                    to: Route::listing(None, Sort::default(), 1),
                    class: "flex items-center gap-1.5 text-sm font-semibold text-teal-700 hover:text-teal-900",
                    "View all"
                    ArrowRight {}
                }
            }
            div { class: "mt-8 grid grid-cols-2 gap-x-6 gap-y-10 lg:grid-cols-4",
                for card in &data.featured {
                    ProductCard { key: "{card.id}", card: card.clone() }
                }
            }
        }
    }
}

/// One of the store perks: an icon, a title and a line of text.
#[component]
fn Perk(title: &'static str, text: &'static str, children: Element) -> Element {
    rsx! {
        li { class: "flex gap-4",
            span { class: "flex size-11 shrink-0 items-center justify-center rounded-full bg-teal-50 text-teal-700",
                {children}
            }
            div {
                p { class: "font-semibold text-stone-900", {title} }
                p { class: "mt-1 text-sm text-stone-600", {text} }
            }
        }
    }
}
