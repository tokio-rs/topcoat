use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{error::RouterErrorExt, page, path_param},
    view::view,
};

use crate::{
    app::_components::{breadcrumbs, product_card, rating_stars, review_list, spec_table},
    catalog::{Catalog, Product, format_rating},
};

#[path_param(error = not_found)]
struct ProductId(u32);

#[page]
async fn product_detail(cx: &Cx) -> Result {
    let catalog = app_context::<Catalog>(cx);
    let product = catalog
        .get(*path_param::<ProductId>(cx)?)
        .ok_or_not_found()?;
    let related: Vec<&Product> = product
        .related_ids
        .iter()
        .filter_map(|id| catalog.get(*id))
        .collect();

    view! {
        breadcrumbs(
            category: &product.category,
            category_slug: &product.category_slug,
            name: &product.name
        )
        <div class="mt-6 grid gap-10 lg:grid-cols-2">
            <div
                class="flex min-h-80 items-center justify-center rounded-2xl bg-slate-100 text-7xl font-bold text-slate-300"
            >
                (product.initials())
            </div>
            <div>
                <p class="text-sm font-medium uppercase tracking-wide text-slate-400">
                    (&product.category)
                </p>
                <h1 class="mt-1 text-3xl font-bold tracking-tight">(&product.name)</h1>
                <div class="mt-3 flex items-center gap-2">
                    rating_stars(tenths: product.rating_tenths, size: "h-5 w-5")
                    <span class="text-sm text-slate-500">
                        (format_rating(product.rating_tenths))
                        " ("
                        (product.review_count)
                        " reviews)"
                    </span>
                </div>
                <p class="mt-6 text-4xl font-bold">(product.price())</p>
                <div class="mt-6 space-y-4 text-slate-600">
                    for paragraph in &product.description {
                        <p>(paragraph)</p>
                    }
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
            spec_table(specs: &product.specs)
        </section>
        <section class="mt-16">
            <h2 class="text-2xl font-bold tracking-tight">
                "Reviews ("
                (product.review_count)
                ")"
            </h2>
            review_list(reviews: &product.reviews)
        </section>
        <section class="mt-16">
            <h2 class="text-2xl font-bold tracking-tight">"Related products"</h2>
            <div class="mt-6 grid grid-cols-2 gap-6 md:grid-cols-4">
                for product in &related {
                    product_card(product: product)
                }
            </div>
        </section>
    }
}
