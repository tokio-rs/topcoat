use leptos::prelude::*;

use super::Palette;
use crate::model::Color;

/// The square product tile with the sale and sold-out badges.
///
/// The dataset has no photos, so the tile draws a soft shape in the product's color instead.
#[component]
pub fn ProductImage(color: Color, on_sale: bool, sold_out: bool) -> impl IntoView {
    let palette = Palette::of(color);
    view! {
        <div class=format!("relative aspect-square overflow-hidden rounded-2xl {}", palette.tile)>
            <div class=format!(
                "absolute left-1/2 top-1/2 aspect-square w-2/5 -translate-x-1/2 -translate-y-1/2 rotate-6 rounded-[28%] shadow-xl transition group-hover:rotate-12 {}",
                palette.accent,
            )></div>
            {(on_sale || sold_out)
                .then(|| {
                    view! {
                        <div class="absolute left-3 top-3 flex gap-1.5">
                            {sold_out
                                .then(|| {
                                    view! {
                                        <span class="rounded-full bg-stone-900 px-2.5 py-1 text-xs font-medium text-white">
                                            "Sold out"
                                        </span>
                                    }
                                })}
                            {on_sale
                                .then(|| {
                                    view! {
                                        <span class="rounded-full bg-rose-600 px-2.5 py-1 text-xs font-medium text-white">
                                            "Sale"
                                        </span>
                                    }
                                })}
                        </div>
                    }
                })}
        </div>
    }
}
