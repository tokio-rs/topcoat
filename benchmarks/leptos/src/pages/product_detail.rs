use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::{
    components::{BreadcrumbsNav, ProductCard, RatingStars, ReviewList, SpecTable},
    format::{format_price, format_rating, initials},
    model::ProductDetailData,
    server_fns::get_product,
};

#[component]
pub fn ProductDetailPage() -> impl IntoView {
    let params = use_params_map();
    let data = Resource::new(
        move || params.get().get("id").and_then(|id| id.parse::<u32>().ok()),
        |id| async move {
            match id {
                Some(id) => get_product(id).await,
                None => Ok(None),
            }
        },
    );

    view! {
        <Suspense fallback=|| ()>
            {move || Suspend::new(async move {
                data.await.ok().flatten().map(product_view)
            })}
        </Suspense>
    }
}

fn product_view(product: ProductDetailData) -> impl IntoView {
    let ProductDetailData {
        id: _,
        name,
        category,
        category_slug,
        price_cents,
        rating_tenths,
        review_count,
        specs,
        description,
        reviews,
        related,
    } = product;
    let initials = initials(&name);

    view! {
        <BreadcrumbsNav category=category.clone() category_slug name=name.clone()/>
        <div class="mt-6 grid gap-10 lg:grid-cols-2">
            <div class="flex min-h-80 items-center justify-center rounded-2xl bg-slate-100 text-7xl font-bold text-slate-300">
                {initials}
            </div>
            <div>
                <p class="text-sm font-medium uppercase tracking-wide text-slate-400">{category}</p>
                <h1 class="mt-1 text-3xl font-bold tracking-tight">{name}</h1>
                <div class="mt-3 flex items-center gap-2">
                    <RatingStars tenths=rating_tenths size="h-5 w-5"/>
                    <span class="text-sm text-slate-500">
                        {format!("{} ({review_count} reviews)", format_rating(rating_tenths))}
                    </span>
                </div>
                <p class="mt-6 text-4xl font-bold">{format_price(price_cents)}</p>
                <div class="mt-6 space-y-4 text-slate-600">
                    {description
                        .into_iter()
                        .map(|paragraph| view! { <p>{paragraph}</p> })
                        .collect_view()}
                </div>
                <button
                    type="button"
                    class="mt-8 inline-block rounded-lg bg-indigo-600 px-8 py-3 text-sm font-semibold text-white"
                >
                    "Add to cart"
                </button>
            </div>
        </div>
        <section class="mt-16">
            <h2 class="text-2xl font-bold tracking-tight">"Specifications"</h2>
            <SpecTable specs/>
        </section>
        <section class="mt-16">
            <h2 class="text-2xl font-bold tracking-tight">{format!("Reviews ({review_count})")}</h2>
            <ReviewList reviews/>
        </section>
        <section class="mt-16">
            <h2 class="text-2xl font-bold tracking-tight">"Related products"</h2>
            <div class="mt-6 grid grid-cols-2 gap-6 md:grid-cols-4">
                {related
                    .into_iter()
                    .map(|product| view! { <ProductCard product/> })
                    .collect_view()}
            </div>
        </section>
    }
}
