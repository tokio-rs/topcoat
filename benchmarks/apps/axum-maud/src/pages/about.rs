use maud::{Markup, html};

use crate::{cart::CartCount, views::layout};

const FACTS: [(&str, &str); 4] = [
    ("Founded", "2016"),
    ("Workshops", "38"),
    ("Repairs last year", "2,140"),
    ("Orders shipped", "96,000+"),
];

/// `GET /about`: a small, static page in the shared layout.
pub async fn about_page(cart: CartCount) -> Markup {
    let content = html! {
        div class="mx-auto max-w-3xl px-4 py-16 sm:px-6 lg:px-8" {
            p class="text-sm font-semibold uppercase tracking-widest text-teal-700" { "Our story" }
            h1 class="mt-3 text-4xl font-semibold tracking-tight text-balance" { "About Harbor Market" }
            p class="mt-6 text-lg leading-8 text-stone-600" {
                "Harbor Market started in 2016 as a weekend stall at the Portland waterfront, selling camp mugs and wool blankets to people on their way to the coast. Today we work with small workshops around the world to make gear that is built to last and easy to repair."
            }
            dl class="mt-12 grid grid-cols-2 gap-4 sm:grid-cols-4" {
                @for (label, value) in FACTS {
                    div class="rounded-2xl border border-stone-200 bg-white p-5" {
                        dt class="text-sm text-stone-500" { (label) }
                        dd class="mt-2 text-2xl font-semibold tracking-tight text-stone-900" { (value) }
                    }
                }
            }
            div class="mt-16 space-y-12 text-stone-600" {
                section id="shipping" {
                    h2 class="text-2xl font-semibold tracking-tight text-stone-900" { "Shipping & returns" }
                    p class="mt-4 leading-7" {
                        "Orders placed before 2 p.m. Pacific ship the same business day. Shipping is free on orders over $75; below that, it is a flat $7.95 anywhere in the contiguous United States."
                    }
                    p class="mt-4 leading-7" {
                        "If something isn't right, send it back within 30 days for a full refund. Return shipping is on us, and we will never charge a restocking fee."
                    }
                }
                section id="warranty" {
                    h2 class="text-2xl font-semibold tracking-tight text-stone-900" { "Warranty & repairs" }
                    p class="mt-4 leading-7" {
                        "Everything we sell is covered against defects in materials and workmanship for at least one year, and most products for much longer. When something wears out, we would rather fix it than replace it."
                    }
                    ul class="mt-4 list-disc space-y-2 pl-5" {
                        li { "Free repairs for the full warranty period." }
                        li { "Spare parts at cost for as long as we sell a product." }
                        li { "Repair guides for common fixes, written by our workshop team." }
                    }
                }
                section id="contact" {
                    h2 class="text-2xl font-semibold tracking-tight text-stone-900" { "Contact us" }
                    p class="mt-4 leading-7" {
                        "Our support team answers every message within one business day, Monday to Friday, 8 a.m. to 6 p.m. Pacific."
                    }
                    dl class="mt-6 grid gap-4 sm:grid-cols-2" {
                        div class="rounded-2xl bg-white p-5 ring-1 ring-stone-200" {
                            dt class="text-sm font-semibold text-stone-900" { "Email" }
                            dd class="mt-1" { "help@harbor-market.example" }
                        }
                        div class="rounded-2xl bg-white p-5 ring-1 ring-stone-200" {
                            dt class="text-sm font-semibold text-stone-900" { "Visit" }
                            dd class="mt-1" { "1200 NW Front Ave, Portland, OR 97209" }
                        }
                    }
                }
            }
        }
    };
    layout(Some("About"), cart, content)
}
