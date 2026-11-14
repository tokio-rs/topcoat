use topcoat::{
    Result,
    context::Cx,
    router::href,
    view::{View, class, component, view},
};

use super::{
    palette::Palette,
    product_image::product_image,
    stars::{StarSize, filled_stars, stars},
};
use crate::{
    app::products::product_id,
    data::{Product, store},
    format::{Count, Money, Rating},
};

/// A product in a grid: tile, category, name, tagline, rating, price and color.
#[component]
pub async fn product_card(cx: &Cx, product: &Product) -> Result<impl View> {
    let category = store(cx).category_of(product);
    let rating = Rating(product.rating).to_string();
    Ok(view! {
        <article class="group relative flex flex-col">
            product_image(product: product)
            <div class="mt-4 flex flex-1 flex-col">
                <p class="text-xs font-medium uppercase tracking-wide text-stone-500">
                    (&category.name)
                </p>
                <h3
                    class="mt-1 text-sm font-semibold text-stone-900 group-hover:text-teal-800"
                >
                    <a href=(href!(product_id::detail, product_id: product.id))>
                        <span class="absolute inset-0"></span>
                        (&product.name)
                    </a>
                </h3>
                <p class="mt-1 line-clamp-2 text-sm text-stone-600">
                    (&product.tagline)
                </p>
                <div class="mt-2 flex items-center gap-1.5">
                    stars(
                        filled: filled_stars(product.rating_tenths()),
                        rating: &rating,
                        size: StarSize::Small
                    )
                    <span class="text-xs text-stone-500">
                        (&rating)
                        " ("
                        (Count(product.review_count.into()).to_string())
                        ")"
                    </span>
                </div>
                <div class="mt-auto flex items-end justify-between gap-2 pt-3">
                    <p class="flex items-baseline gap-2">
                        <span class="font-semibold text-stone-900">
                            (Money(product.price_cents.into()).to_string())
                        </span>
                        if let Some(compare_at) = product.compare_at_cents {
                            <s class="text-sm text-stone-400">
                                (Money(compare_at.into()).to_string())
                            </s>
                        }
                    </p>
                    <span class="flex items-center gap-1.5 text-xs text-stone-500">
                        <span
                            class=(class!(
                                "size-3 rounded-full ring-1 ring-black/10",
                                Palette::of(product.color).accent,
                            ))
                        ></span>
                        (product.color.label())
                    </span>
                </div>
                if (1..10).contains(&product.stock) {
                    <p class="mt-1 text-xs font-medium text-amber-700">
                        "Only "
                        (product.stock)
                        " left"
                    </p>
                }
            </div>
        </article>
    })
}
