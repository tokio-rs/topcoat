use std::sync::Arc;

use axum::extract::State;
use maud::{Markup, html};

use crate::{
    cart::CartCount,
    data::Store,
    format::{Count, IsoDateTime, Money, ShortDateTime},
    views::{layout, status_badge},
};

const COLUMNS: [&str; 8] = [
    "Order", "Placed", "Customer", "Email", "Country", "Items", "Status", "Total",
];

/// `GET /admin/orders`: summary cards and a table of every order, newest first.
pub async fn orders_page(State(store): State<Arc<Store>>, cart: CartCount) -> Markup {
    let summary = store.order_summary();
    let stats = [
        (
            "Orders",
            Count(summary.count as u64).to_string(),
            "All time",
        ),
        (
            "Revenue",
            Money(summary.revenue_cents).to_string(),
            "Paid, shipped and delivered",
        ),
        (
            "Average order",
            Money(summary.average_cents).to_string(),
            "Across revenue orders",
        ),
        (
            "Refunded",
            Money(summary.refunded_cents).to_string(),
            "Returned to customers",
        ),
    ];
    let content = html! {
        div class="mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8" {
            p class="text-sm font-semibold uppercase tracking-widest text-teal-700" { "Admin" }
            h1 class="mt-2 text-3xl font-semibold tracking-tight sm:text-4xl" { "Orders" }
            p class="mt-2 text-stone-600" { "All " (Count(summary.count as u64)) " orders, newest first." }
            dl class="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4" {
                @for (label, value, note) in &stats {
                    div class="rounded-2xl border border-stone-200 bg-white p-5" {
                        dt class="text-sm font-medium text-stone-500" { (label) }
                        dd class="mt-2 text-2xl font-semibold tracking-tight text-stone-900" { (value) }
                        dd class="mt-1 text-xs text-stone-500" { (note) }
                    }
                }
            }
            div class="mt-8 overflow-x-auto rounded-2xl border border-stone-200 bg-white" {
                table class="min-w-full whitespace-nowrap text-left text-sm" {
                    thead class="border-b border-stone-200 bg-stone-50 text-xs font-semibold uppercase tracking-wide text-stone-500" {
                        tr {
                            @for column in COLUMNS {
                                @if column == "Items" || column == "Total" {
                                    th scope="col" class="px-4 py-3 text-right" { (column) }
                                } @else {
                                    th scope="col" class="px-4 py-3" { (column) }
                                }
                            }
                        }
                    }
                    tbody class="divide-y divide-stone-100 text-stone-600" {
                        @for order in store.orders() {
                            tr class="hover:bg-stone-50" {
                                td class="px-4 py-3 font-medium text-stone-900" { (order.number) }
                                td class="px-4 py-3" {
                                    time datetime=(IsoDateTime(order.placed_at)) { (ShortDateTime(order.placed_at)) }
                                }
                                td class="px-4 py-3 text-stone-900" { (order.customer) }
                                td class="px-4 py-3 text-stone-500" { (order.email) }
                                td class="px-4 py-3" { (order.country) }
                                td class="px-4 py-3 text-right tabular-nums" { (order.items) }
                                td class="px-4 py-3" { (status_badge(order.status)) }
                                td class="px-4 py-3 text-right font-medium tabular-nums text-stone-900" { (Money(order.total_cents)) }
                            }
                        }
                    }
                }
            }
        }
    };
    layout(Some("Orders"), cart, content)
}
