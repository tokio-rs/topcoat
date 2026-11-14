use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use super::error_page;
use crate::{
    cart::CartCount,
    format::{Count, Money, Rating, iso_date, short_date},
    model::{self, Category, Product, Sort},
    routes::Route,
    views::{
        Breadcrumbs, Crumb, Palette, ProductCard, ProductImage, StarSize, Stars, filled_stars,
        icons::{Check, Minus, Plus},
    },
};

const STEPPER_BUTTON: &str = "rounded-full p-2.5 text-stone-700 hover:bg-stone-100 disabled:cursor-not-allowed disabled:opacity-30 disabled:hover:bg-transparent";

/// The data of a product detail page.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct DetailData {
    product: Product,
    category: Category,
    related: Vec<model::ProductCard>,
}

/// Loads a product with its category and related products. Fails with 404 for an unknown id.
#[server]
async fn detail_data(id: u32) -> ServerFnResult<DetailData> {
    let store = crate::store::Store::get();
    let product = store.product(id).or_not_found("product not found")?;
    Ok(DetailData {
        product: product.clone(),
        category: store.category_of(product).clone(),
        related: store
            .related(product)
            .map(|related| store.card(related))
            .collect(),
    })
}

/// `GET /products/{id}`: product details, specifications, reviews and related products.
#[component]
pub fn Detail(id: u32) -> Element {
    let data = use_server_future(use_reactive!(|id| detail_data(id)))?;
    let data = data.read();
    match &*data {
        Some(Ok(data)) => detail(data),
        Some(Err(error)) => error_page(error),
        None => VNode::empty(),
    }
}

fn detail(data: &DetailData) -> Element {
    let DetailData {
        product,
        category,
        related,
    } = data;
    let category_route = Route::category(&category.slug);
    let rating = Rating(product.rating).to_string();
    let filled = filled_stars(product.rating_tenths());
    let review_count = Count(product.review_count.into());
    let price = Money(product.price_cents.into());
    let swatch = Palette::of(product.color).accent;
    let trail = vec![
        Crumb::new("Home", Route::Home {}),
        Crumb::new("Products", Route::listing(None, Sort::default(), 1)),
        Crumb::new(&category.name, category_route.clone()),
    ];

    rsx! {
        document::Title { "{product.name} | Harbor Market" }
        div { class: "mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8",
            Breadcrumbs { trail, current: &product.name }
            div { class: "mt-8 grid gap-10 lg:grid-cols-2 lg:gap-16",
                div { class: "lg:sticky lg:top-24 lg:self-start",
                    ProductImage {
                        color: product.color,
                        on_sale: product.compare_at_cents.is_some(),
                        sold_out: product.stock == 0,
                    }
                }
                div {
                    Link {
                        to: category_route,
                        class: "text-sm font-semibold text-teal-700 hover:text-teal-900",
                        "{category.name}"
                    }
                    h1 { class: "mt-2 text-3xl font-semibold tracking-tight text-balance text-stone-900 sm:text-4xl",
                        "{product.name}"
                    }
                    div { class: "mt-3 flex items-center gap-2",
                        Stars {
                            filled,
                            rating: rating.clone(),
                            size: StarSize::Large,
                        }
                        span { class: "text-sm font-medium text-stone-900", "{rating}" }
                        a {
                            href: "#reviews",
                            class: "text-sm text-stone-500 underline-offset-4 hover:text-stone-900 hover:underline",
                            "{review_count} reviews"
                        }
                    }
                    div { class: "mt-6 flex flex-wrap items-baseline gap-3",
                        p { class: "text-3xl font-semibold tracking-tight text-stone-900",
                            "{price}"
                        }
                        if let Some(compare_at) = product.compare_at_cents {
                            Savings { price: product.price_cents, compare_at }
                        }
                    }
                    p { class: "mt-4 text-lg leading-8 text-stone-600", "{product.tagline}" }
                    dl { class: "mt-6 grid grid-cols-[auto_1fr] gap-x-8 gap-y-3 text-sm",
                        dt { class: "font-medium text-stone-900", "Color" }
                        dd { class: "flex items-center gap-2 text-stone-600",
                            span { class: "size-4 rounded-full ring-1 ring-black/10 {swatch}" }
                            {product.color.label()}
                        }
                        dt { class: "font-medium text-stone-900", "Availability" }
                        match product.stock {
                            0 => rsx! {
                                dd { class: "font-medium text-stone-500", "Sold out" }
                            },
                            stock @ 1..10 => rsx! {
                                dd { class: "font-medium text-amber-700", "Only {stock} left in stock" }
                            },
                            _ => rsx! {
                                dd { class: "font-medium text-emerald-700", "In stock" }
                            },
                        }
                    }
                    if product.stock == 0 {
                        div { class: "mt-8",
                            button {
                                r#type: "button",
                                disabled: true,
                                class: "w-full cursor-not-allowed rounded-full bg-stone-200 px-8 py-3.5 text-base font-semibold text-stone-500",
                                "Sold out"
                            }
                        }
                    } else {
                        PurchaseBox { max: product.max_quantity() }
                    }
                    ul { class: "mt-8 space-y-3 border-t border-stone-200 pt-6 text-sm text-stone-600",
                        li { class: "flex items-center gap-3",
                            Check {}
                            "Free shipping on orders over $75"
                        }
                        li { class: "flex items-center gap-3",
                            Check {}
                            "Free 30-day returns"
                        }
                        li { class: "flex items-center gap-3",
                            Check {}
                            "Ships within one business day"
                        }
                    }
                }
            }
            div { class: "mt-16 grid gap-12 border-t border-stone-200 pt-16 lg:grid-cols-2",
                section {
                    h2 { class: "text-xl font-semibold tracking-tight", "Description" }
                    div { class: "mt-4 space-y-4 leading-7 text-stone-600",
                        for paragraph in product.paragraphs() {
                            p { {paragraph} }
                        }
                    }
                }
                section {
                    h2 { class: "text-xl font-semibold tracking-tight", "Specifications" }
                    table { class: "mt-4 w-full text-left text-sm",
                        tbody { class: "divide-y divide-stone-200 border-y border-stone-200",
                            for spec in &product.specs {
                                tr {
                                    th {
                                        scope: "row",
                                        class: "w-2/5 py-3 pr-4 font-medium text-stone-900",
                                        "{spec.label}"
                                    }
                                    td { class: "py-3 text-stone-600", "{spec.value}" }
                                }
                            }
                        }
                    }
                }
            }
            section {
                id: "reviews",
                class: "mt-16 grid gap-10 border-t border-stone-200 pt-16 lg:grid-cols-[18rem_1fr]",
                div {
                    h2 { class: "text-xl font-semibold tracking-tight", "Customer reviews" }
                    div { class: "mt-4 flex items-center gap-3",
                        p { class: "text-4xl font-semibold tracking-tight", "{rating}" }
                        Stars {
                            filled,
                            rating: rating.clone(),
                            size: StarSize::Large,
                        }
                    }
                    p { class: "mt-2 text-sm text-stone-600", "Based on {review_count} reviews" }
                    p { class: "mt-1 text-sm text-stone-500",
                        "Showing the {product.reviews.len()} most recent."
                    }
                }
                ul { class: "divide-y divide-stone-200",
                    for review in &product.reviews {
                        li { class: "py-6 first:pt-0",
                            article {
                                div { class: "flex items-center justify-between gap-4",
                                    Stars {
                                        filled: review.rating,
                                        rating: review.rating.to_string(),
                                        size: StarSize::Small,
                                    }
                                    time {
                                        datetime: iso_date(review.date),
                                        class: "text-sm text-stone-500",
                                        {short_date(review.date)}
                                    }
                                }
                                h3 { class: "mt-3 font-semibold text-stone-900", "{review.title}" }
                                p { class: "mt-2 leading-7 text-stone-600", "{review.body}" }
                                p { class: "mt-3 text-sm font-medium text-stone-900", "{review.author}" }
                            }
                        }
                    }
                }
            }
            section {
                aria_labelledby: "related-heading",
                class: "mt-16 border-t border-stone-200 pt-16",
                h2 { id: "related-heading", class: "text-xl font-semibold tracking-tight",
                    "You might also like"
                }
                div { class: "mt-8 grid grid-cols-2 gap-x-6 gap-y-10 lg:grid-cols-4",
                    for card in related {
                        ProductCard { key: "{card.id}", card: card.clone() }
                    }
                }
            }
        }
    }
}

/// The compare-at price and how much the sale saves.
#[component]
fn Savings(price: u32, compare_at: u32) -> Element {
    let saved = compare_at - price;
    let percent = saved * 100 / compare_at;
    rsx! {
        s { class: "text-lg text-stone-400", "{Money(compare_at.into())}" }
        span { class: "rounded-full bg-rose-50 px-2.5 py-1 text-sm font-medium text-rose-700",
            "Save {Money(saved.into())} ({percent}%)"
        }
    }
}

/// The quantity stepper and add-to-cart button of an in-stock product.
///
/// Adding to the cart updates the `cart_count` cookie and the [`CartCount`] signal that the header
/// badge shows.
#[component]
fn PurchaseBox(max: u32) -> Element {
    let mut quantity = use_signal(|| 1);
    let mut status = use_signal(String::new);
    let mut cart = use_context::<Signal<CartCount>>();

    let add_to_cart = move |_| {
        let count = CartCount::read_cookie().add(quantity());
        count.write_cookie();
        cart.set(count);
        status.set(format!("Added {quantity} to your cart."));
    };

    rsx! {
        div { class: "mt-8",
            div { class: "flex flex-col gap-3 sm:flex-row",
                div { class: "flex items-center justify-between rounded-full border border-stone-300 bg-white p-1 sm:w-36",
                    button {
                        r#type: "button",
                        aria_label: "Decrease quantity",
                        disabled: quantity() <= 1,
                        class: STEPPER_BUTTON,
                        onclick: move |_| quantity.set((quantity() - 1).max(1)),
                        Minus {}
                    }
                    output {
                        aria_live: "polite",
                        aria_label: "Quantity",
                        class: "w-10 text-center font-semibold tabular-nums",
                        "{quantity}"
                    }
                    button {
                        r#type: "button",
                        aria_label: "Increase quantity",
                        disabled: quantity() >= max,
                        class: STEPPER_BUTTON,
                        onclick: move |_| quantity.set((quantity() + 1).min(max)),
                        Plus {}
                    }
                }
                button {
                    r#type: "button",
                    class: "flex-1 rounded-full bg-teal-700 px-8 py-3.5 text-base font-semibold text-white shadow-sm hover:bg-teal-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-teal-700",
                    onclick: add_to_cart,
                    "Add to cart"
                }
            }
            p {
                role: "status",
                class: "mt-3 min-h-5 text-sm font-medium text-teal-800",
                "{status}"
            }
        }
    }
}
