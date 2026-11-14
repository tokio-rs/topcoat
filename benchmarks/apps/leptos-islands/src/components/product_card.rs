use leptos::prelude::*;

use super::{Palette, ProductImage, StarSize, Stars, filled_stars};
use crate::{
    format::{Count, Money, Rating},
    model::ProductSummary,
    urls::ProductUrl,
};

/// A product in a grid: tile, category, name, tagline, rating, price and color.
#[component]
pub fn ProductCard(product: ProductSummary) -> impl IntoView {
    let rating = Rating(product.rating).to_string();
    let filled = filled_stars(product.rating_tenths());
    let stock = product.stock;
    view! {
        <article class="group relative flex flex-col">
            <ProductImage
                color=product.color
                on_sale=product.compare_at_cents.is_some()
                sold_out=product.is_sold_out()
            />
            <div class="mt-4 flex flex-1 flex-col">
                <p class="text-xs font-medium uppercase tracking-wide text-stone-500">
                    {product.category_name}
                </p>
                <h3 class="mt-1 text-sm font-semibold text-stone-900 group-hover:text-teal-800">
                    <a href=ProductUrl(product.id).to_string()>
                        <span class="absolute inset-0"></span>
                        {product.name}
                    </a>
                </h3>
                <p class="mt-1 line-clamp-2 text-sm text-stone-600">{product.tagline}</p>
                <div class="mt-2 flex items-center gap-1.5">
                    <Stars
                        filled
                        rating=rating.clone()
                        size=StarSize::Small
                    />
                    <span class="text-xs text-stone-500">
                        {format!("{rating} ({})", Count(product.review_count.into()))}
                    </span>
                </div>
                <div class="mt-auto flex items-end justify-between gap-2 pt-3">
                    <p class="flex items-baseline gap-2">
                        <span class="font-semibold text-stone-900">
                            {Money(product.price_cents.into()).to_string()}
                        </span>
                        {product
                            .compare_at_cents
                            .map(|compare_at| {
                                view! {
                                    <s class="text-sm text-stone-400">
                                        {Money(compare_at.into()).to_string()}
                                    </s>
                                }
                            })}
                    </p>
                    <span class="flex items-center gap-1.5 text-xs text-stone-500">
                        <span class=format!(
                            "size-3 rounded-full ring-1 ring-black/10 {}",
                            Palette::of(product.color).accent,
                        )></span>
                        {product.color.label()}
                    </span>
                </div>
                {(1..10)
                    .contains(&stock)
                    .then(|| {
                        view! {
                            <p class="mt-1 text-xs font-medium text-amber-700">
                                {format!("Only {stock} left")}
                            </p>
                        }
                    })}
            </div>
        </article>
    }
}
