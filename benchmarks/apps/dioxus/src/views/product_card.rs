use dioxus::prelude::*;

use super::{Palette, ProductImage, StarSize, Stars, filled_stars};
use crate::{
    format::{Count, Money, Rating},
    model::ProductCard as Card,
    routes::Route,
};

/// A product in a grid: tile, category, name, tagline, rating, price and color.
#[component]
pub fn ProductCard(card: Card) -> Element {
    let rating = Rating(card.rating).to_string();
    let review_count = Count(card.review_count.into());
    let price = Money(card.price_cents.into());
    let compare_at = card.compare_at_cents.map(|cents| Money(cents.into()));
    let filled = filled_stars(card.rating_tenths());
    let swatch = Palette::of(card.color).accent;
    rsx! {
        article { class: "group relative flex flex-col",
            ProductImage {
                color: card.color,
                on_sale: card.compare_at_cents.is_some(),
                sold_out: card.stock == 0,
            }
            div { class: "mt-4 flex flex-1 flex-col",
                p { class: "text-xs font-medium uppercase tracking-wide text-stone-500",
                    {card.category_name}
                }
                h3 { class: "mt-1 text-sm font-semibold text-stone-900 group-hover:text-teal-800",
                    Link { to: Route::Detail { id: card.id },
                        span { class: "absolute inset-0" }
                        {card.name}
                    }
                }
                p { class: "mt-1 line-clamp-2 text-sm text-stone-600", {card.tagline} }
                div { class: "mt-2 flex items-center gap-1.5",
                    Stars {
                        filled,
                        rating: rating.clone(),
                        size: StarSize::Small,
                    }
                    span { class: "text-xs text-stone-500", "{rating} ({review_count})" }
                }
                div { class: "mt-auto flex items-end justify-between gap-2 pt-3",
                    p { class: "flex items-baseline gap-2",
                        span { class: "font-semibold text-stone-900", "{price}" }
                        if let Some(compare_at) = compare_at {
                            s { class: "text-sm text-stone-400", "{compare_at}" }
                        }
                    }
                    span { class: "flex items-center gap-1.5 text-xs text-stone-500",
                        span { class: "size-3 rounded-full ring-1 ring-black/10 {swatch}" }
                        {card.color.label()}
                    }
                }
                if (1..10).contains(&card.stock) {
                    p { class: "mt-1 text-xs font-medium text-amber-700", "Only {card.stock} left" }
                }
            }
        }
    }
}
