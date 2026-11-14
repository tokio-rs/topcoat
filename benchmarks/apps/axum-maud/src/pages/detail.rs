use std::sync::Arc;

use axum::extract::{Path, State};
use maud::{Markup, html};

use super::ErrorPage;
use crate::{
    cart::CartCount,
    data::{Product, Store},
    format::{Count, IsoDate, Money, Rating, ShortDate},
    urls::ListingUrl,
    views::{
        Palette, StarSize, breadcrumbs, filled_stars, icons, layout, product_card, product_image,
        stars,
    },
};

const STEPPER_BUTTON: &str = "rounded-full p-2.5 text-stone-700 hover:bg-stone-100 disabled:cursor-not-allowed disabled:opacity-30 disabled:hover:bg-transparent";

/// `GET /products/{id}`: product details, specifications, reviews and related products.
pub async fn detail_page(
    State(store): State<Arc<Store>>,
    cart: CartCount,
    Path(id): Path<String>,
) -> Result<Markup, ErrorPage> {
    let product = id
        .parse()
        .ok()
        .and_then(|id| store.product(id))
        .ok_or(ErrorPage::not_found(cart))?;
    let category = store.category_of(product);
    let category_href = ListingUrl::category(&category.slug).to_string();
    let rating = Rating(product.rating);

    let content = html! {
        div class="mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8" {
            (breadcrumbs(&[("Home", "/"), ("Products", "/products"), (&category.name, &category_href)], &product.name))
            div class="mt-8 grid gap-10 lg:grid-cols-2 lg:gap-16" {
                div class="lg:sticky lg:top-24 lg:self-start" {
                    (product_image(product))
                }
                div {
                    a href=(category_href) class="text-sm font-semibold text-teal-700 hover:text-teal-900" { (category.name) }
                    h1 class="mt-2 text-3xl font-semibold tracking-tight text-balance text-stone-900 sm:text-4xl" { (product.name) }
                    div class="mt-3 flex items-center gap-2" {
                        (stars(filled_stars(product.rating_tenths()), &rating, StarSize::Large))
                        span class="text-sm font-medium text-stone-900" { (rating) }
                        a href="#reviews" class="text-sm text-stone-500 underline-offset-4 hover:text-stone-900 hover:underline" {
                            (Count(product.review_count.into())) " reviews"
                        }
                    }
                    div class="mt-6 flex flex-wrap items-baseline gap-3" {
                        p class="text-3xl font-semibold tracking-tight text-stone-900" { (Money(product.price_cents.into())) }
                        @if let Some(compare_at) = product.compare_at_cents {
                            s class="text-lg text-stone-400" { (Money(compare_at.into())) }
                            span class="rounded-full bg-rose-50 px-2.5 py-1 text-sm font-medium text-rose-700" {
                                "Save " (Money((compare_at - product.price_cents).into()))
                                " (" ((compare_at - product.price_cents) * 100 / compare_at) "%)"
                            }
                        }
                    }
                    p class="mt-4 text-lg leading-8 text-stone-600" { (product.tagline) }
                    dl class="mt-6 grid grid-cols-[auto_1fr] gap-x-8 gap-y-3 text-sm" {
                        dt class="font-medium text-stone-900" { "Color" }
                        dd class="flex items-center gap-2 text-stone-600" {
                            span class={ "size-4 rounded-full ring-1 ring-black/10 " (Palette::of(product.color).accent) } {}
                            (product.color.label())
                        }
                        dt class="font-medium text-stone-900" { "Availability" }
                        (availability(product))
                    }
                    (purchase(product))
                    ul class="mt-8 space-y-3 border-t border-stone-200 pt-6 text-sm text-stone-600" {
                        li class="flex items-center gap-3" { (icons::check()) "Free shipping on orders over $75" }
                        li class="flex items-center gap-3" { (icons::check()) "Free 30-day returns" }
                        li class="flex items-center gap-3" { (icons::check()) "Ships within one business day" }
                    }
                }
            }
            div class="mt-16 grid gap-12 border-t border-stone-200 pt-16 lg:grid-cols-2" {
                section {
                    h2 class="text-xl font-semibold tracking-tight" { "Description" }
                    div class="mt-4 space-y-4 leading-7 text-stone-600" {
                        @for paragraph in product.paragraphs() {
                            p { (paragraph) }
                        }
                    }
                }
                section {
                    h2 class="text-xl font-semibold tracking-tight" { "Specifications" }
                    table class="mt-4 w-full text-left text-sm" {
                        tbody class="divide-y divide-stone-200 border-y border-stone-200" {
                            @for spec in &product.specs {
                                tr {
                                    th scope="row" class="w-2/5 py-3 pr-4 font-medium text-stone-900" { (spec.label) }
                                    td class="py-3 text-stone-600" { (spec.value) }
                                }
                            }
                        }
                    }
                }
            }
            section id="reviews" class="mt-16 grid gap-10 border-t border-stone-200 pt-16 lg:grid-cols-[18rem_1fr]" {
                div {
                    h2 class="text-xl font-semibold tracking-tight" { "Customer reviews" }
                    div class="mt-4 flex items-center gap-3" {
                        p class="text-4xl font-semibold tracking-tight" { (rating) }
                        (stars(filled_stars(product.rating_tenths()), &rating, StarSize::Large))
                    }
                    p class="mt-2 text-sm text-stone-600" { "Based on " (Count(product.review_count.into())) " reviews" }
                    p class="mt-1 text-sm text-stone-500" { "Showing the " (product.reviews.len()) " most recent." }
                }
                ul class="divide-y divide-stone-200" {
                    @for review in &product.reviews {
                        li class="py-6 first:pt-0" {
                            article {
                                div class="flex items-center justify-between gap-4" {
                                    (stars(review.rating, review.rating, StarSize::Small))
                                    time datetime=(IsoDate(review.date)) class="text-sm text-stone-500" { (ShortDate(review.date)) }
                                }
                                h3 class="mt-3 font-semibold text-stone-900" { (review.title) }
                                p class="mt-2 leading-7 text-stone-600" { (review.body) }
                                p class="mt-3 text-sm font-medium text-stone-900" { (review.author) }
                            }
                        }
                    }
                }
            }
            section aria-labelledby="related-heading" class="mt-16 border-t border-stone-200 pt-16" {
                h2 id="related-heading" class="text-xl font-semibold tracking-tight" { "You might also like" }
                div class="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 lg:grid-cols-4" {
                    @for related in store.related(product) {
                        (product_card(&store, related))
                    }
                }
            }
        }
    };
    Ok(layout(Some(&product.name), cart, content))
}

fn availability(product: &Product) -> Markup {
    html! {
        @match product.stock {
            0 => dd class="font-medium text-stone-500" { "Sold out" },
            stock @ 1..10 => dd class="font-medium text-amber-700" { "Only " (stock) " left in stock" },
            _ => dd class="font-medium text-emerald-700" { "In stock" },
        }
    }
}

/// The quantity stepper and add-to-cart button, or a disabled button when sold out.
fn purchase(product: &Product) -> Markup {
    let max = product.max_quantity();
    html! {
        @if product.is_sold_out() {
            div class="mt-8" {
                button type="button" disabled class="w-full cursor-not-allowed rounded-full bg-stone-200 px-8 py-3.5 text-base font-semibold text-stone-500" {
                    "Sold out"
                }
            }
        } @else {
            div data-product-form data-max=(max) class="mt-8" {
                div class="flex flex-col gap-3 sm:flex-row" {
                    div class="flex items-center justify-between rounded-full border border-stone-300 bg-white p-1 sm:w-36" {
                        button type="button" data-step="-1" aria-label="Decrease quantity" disabled class=(STEPPER_BUTTON) {
                            (icons::minus())
                        }
                        output data-quantity aria-live="polite" aria-label="Quantity" class="w-10 text-center font-semibold tabular-nums" { "1" }
                        button type="button" data-step="1" aria-label="Increase quantity" disabled[max <= 1] class=(STEPPER_BUTTON) {
                            (icons::plus())
                        }
                    }
                    button type="button" data-add-to-cart class="flex-1 rounded-full bg-teal-700 px-8 py-3.5 text-base font-semibold text-white shadow-sm hover:bg-teal-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-teal-700" {
                        "Add to cart"
                    }
                }
                p data-cart-status role="status" class="mt-3 min-h-5 text-sm font-medium text-teal-800" {}
            }
        }
    }
}
