use topcoat::{
    Result,
    context::Cx,
    view::{View, class, component, view},
};

use super::palette::Palette;
use crate::data::Product;

/// The square product tile with the sale and sold-out badges.
///
/// The dataset has no photos, so the tile draws a soft shape in the product's color instead.
#[component]
pub async fn product_image(cx: &Cx, product: &Product) -> Result<impl View> {
    let palette = Palette::of(product.color);
    let on_sale = product.compare_at_cents.is_some();
    let sold_out = product.is_sold_out();
    Ok(view! {
        <div
            class=(class!(
                "relative aspect-square overflow-hidden rounded-2xl",
                palette.tile,
            ))
        >
            <div
                class=(class!(
                    "absolute left-1/2 top-1/2 aspect-square w-2/5 -translate-x-1/2 -translate-y-1/2 rotate-6 rounded-[28%] shadow-xl transition group-hover:rotate-12",
                    palette.accent,
                ))
            ></div>
            if on_sale || sold_out {
                <div class="absolute left-3 top-3 flex gap-1.5">
                    if sold_out {
                        <span
                            class="rounded-full bg-stone-900 px-2.5 py-1 text-xs font-medium text-white"
                        >
                            "Sold out"
                        </span>
                    }
                    if on_sale {
                        <span
                            class="rounded-full bg-rose-600 px-2.5 py-1 text-xs font-medium text-white"
                        >
                            "Sale"
                        </span>
                    }
                </div>
            }
        </div>
    })
}
