use maud::{Markup, html};

use super::{Palette, StarSize, filled_stars, product_image, stars};
use crate::{
    data::{Product, Store},
    format::{Count, Money, Rating},
    urls::ProductUrl,
};

/// A product in a grid: tile, category, name, tagline, rating, price and color.
pub fn product_card(store: &Store, product: &Product) -> Markup {
    let category = store.category_of(product);
    let rating = Rating(product.rating);
    html! {
        article class="group relative flex flex-col" {
            (product_image(product))
            div class="mt-4 flex flex-1 flex-col" {
                p class="text-xs font-medium uppercase tracking-wide text-stone-500" { (category.name) }
                h3 class="mt-1 text-sm font-semibold text-stone-900 group-hover:text-teal-800" {
                    a href=(ProductUrl(product.id)) {
                        span class="absolute inset-0" {}
                        (product.name)
                    }
                }
                p class="mt-1 line-clamp-2 text-sm text-stone-600" { (product.tagline) }
                div class="mt-2 flex items-center gap-1.5" {
                    (stars(filled_stars(product.rating_tenths()), &rating, StarSize::Small))
                    span class="text-xs text-stone-500" {
                        (rating) " (" (Count(product.review_count.into())) ")"
                    }
                }
                div class="mt-auto flex items-end justify-between gap-2 pt-3" {
                    p class="flex items-baseline gap-2" {
                        span class="font-semibold text-stone-900" { (Money(product.price_cents.into())) }
                        @if let Some(compare_at) = product.compare_at_cents {
                            s class="text-sm text-stone-400" { (Money(compare_at.into())) }
                        }
                    }
                    span class="flex items-center gap-1.5 text-xs text-stone-500" {
                        span class={ "size-3 rounded-full ring-1 ring-black/10 " (Palette::of(product.color).accent) } {}
                        (product.color.label())
                    }
                }
                @if (1..10).contains(&product.stock) {
                    p class="mt-1 text-xs font-medium text-amber-700" { "Only " (product.stock) " left" }
                }
            }
        }
    }
}
