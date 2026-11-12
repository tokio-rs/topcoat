use maud::{Markup, html};

use super::Palette;
use crate::data::Product;

/// The square product tile with the sale and sold-out badges.
///
/// The dataset has no photos, so the tile draws a soft shape in the product's color instead.
pub fn product_image(product: &Product) -> Markup {
    let palette = Palette::of(product.color);
    let on_sale = product.compare_at_cents.is_some();
    let sold_out = product.is_sold_out();
    html! {
        div class={ "relative aspect-square overflow-hidden rounded-2xl " (palette.tile) } {
            div class={ "absolute left-1/2 top-1/2 aspect-square w-2/5 -translate-x-1/2 -translate-y-1/2 rotate-6 rounded-[28%] shadow-xl transition group-hover:rotate-12 " (palette.accent) } {}
            @if on_sale || sold_out {
                div class="absolute left-3 top-3 flex gap-1.5" {
                    @if sold_out {
                        span class="rounded-full bg-stone-900 px-2.5 py-1 text-xs font-medium text-white" { "Sold out" }
                    }
                    @if on_sale {
                        span class="rounded-full bg-rose-600 px-2.5 py-1 text-xs font-medium text-white" { "Sale" }
                    }
                }
            }
        }
    }
}
