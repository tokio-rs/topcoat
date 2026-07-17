use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use maud::{Markup, html};

use crate::{
    catalog::{Catalog, Product, format_rating},
    components::{breadcrumbs, layout, product_card, rating_stars, review_list, spec_table},
};

pub async fn product_detail(
    State(catalog): State<Arc<Catalog>>,
    Path(id): Path<u32>,
) -> Result<Markup, StatusCode> {
    let product = catalog.get(id).ok_or(StatusCode::NOT_FOUND)?;
    let related: Vec<&Product> = product
        .related_ids
        .iter()
        .filter_map(|id| catalog.get(*id))
        .collect();

    Ok(layout(html! {
        (breadcrumbs(&product.category, &product.category_slug, &product.name))
        div class="mt-6 grid gap-10 lg:grid-cols-2" {
            div class="flex min-h-80 items-center justify-center rounded-2xl bg-slate-100 text-7xl font-bold text-slate-300" {
                (product.initials())
            }
            div {
                p class="text-sm font-medium uppercase tracking-wide text-slate-400" {
                    (product.category)
                }
                h1 class="mt-1 text-3xl font-bold tracking-tight" { (product.name) }
                div class="mt-3 flex items-center gap-2" {
                    (rating_stars(product.rating_tenths, "h-5 w-5"))
                    span class="text-sm text-slate-500" {
                        (format_rating(product.rating_tenths)) " (" (product.review_count) " reviews)"
                    }
                }
                p class="mt-6 text-4xl font-bold" { (product.price()) }
                div class="mt-6 space-y-4 text-slate-600" {
                    @for paragraph in &product.description {
                        p { (paragraph) }
                    }
                }
                button type="button"
                    class="mt-8 inline-block rounded-lg bg-indigo-600 px-8 py-3 text-sm font-semibold text-white" {
                    "Add to cart"
                }
            }
        }
        section class="mt-16" {
            h2 class="text-2xl font-bold tracking-tight" { "Specifications" }
            (spec_table(&product.specs))
        }
        section class="mt-16" {
            h2 class="text-2xl font-bold tracking-tight" {
                "Reviews (" (product.review_count) ")"
            }
            (review_list(&product.reviews))
        }
        section class="mt-16" {
            h2 class="text-2xl font-bold tracking-tight" { "Related products" }
            div class="mt-6 grid grid-cols-2 gap-6 md:grid-cols-4" {
                @for product in &related {
                    (product_card(product))
                }
            }
        }
    }))
}
