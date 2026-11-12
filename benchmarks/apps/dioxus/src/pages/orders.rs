use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use super::error_page;
use crate::{
    format::{Count, Money, iso_date_time, short_date_time},
    model::{Order, OrderSummary},
    views::StatusBadge,
};

const COLUMNS: [&str; 8] = [
    "Order", "Placed", "Customer", "Email", "Country", "Items", "Status", "Total",
];

/// The data of the orders page.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct OrdersData {
    summary: OrderSummary,
    /// Every order, newest first.
    orders: Vec<Order>,
}

/// Loads every order with the order totals.
#[server]
async fn orders_data() -> ServerFnResult<OrdersData> {
    let store = crate::store::Store::get();
    Ok(OrdersData {
        summary: store.order_summary().clone(),
        orders: store.orders().to_vec(),
    })
}

/// `GET /admin/orders`: summary cards and a table of every order, newest first.
#[component]
pub fn Orders() -> Element {
    let data = use_server_future(orders_data)?;
    let data = data.read();
    match &*data {
        Some(Ok(data)) => orders(data),
        Some(Err(error)) => error_page(error),
        None => VNode::empty(),
    }
}

fn orders(data: &OrdersData) -> Element {
    let summary = &data.summary;
    let count = Count(summary.count);
    let stats = [
        ("Orders", count.to_string(), "All time"),
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
    rsx! {
        document::Title { "Orders | Harbor Market" }
        div { class: "mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8",
            p { class: "text-sm font-semibold uppercase tracking-widest text-teal-700", "Admin" }
            h1 { class: "mt-2 text-3xl font-semibold tracking-tight sm:text-4xl", "Orders" }
            p { class: "mt-2 text-stone-600", "All {count} orders, newest first." }
            dl { class: "mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4",
                for (label , value , note) in stats {
                    div { class: "rounded-2xl border border-stone-200 bg-white p-5",
                        dt { class: "text-sm font-medium text-stone-500", {label} }
                        dd { class: "mt-2 text-2xl font-semibold tracking-tight text-stone-900",
                            {value}
                        }
                        dd { class: "mt-1 text-xs text-stone-500", {note} }
                    }
                }
            }
            div { class: "mt-8 overflow-x-auto rounded-2xl border border-stone-200 bg-white",
                table { class: "min-w-full whitespace-nowrap text-left text-sm",
                    thead { class: "border-b border-stone-200 bg-stone-50 text-xs font-semibold uppercase tracking-wide text-stone-500",
                        tr {
                            for column in COLUMNS {
                                if column == "Items" || column == "Total" {
                                    th { scope: "col", class: "px-4 py-3 text-right", {column} }
                                } else {
                                    th { scope: "col", class: "px-4 py-3", {column} }
                                }
                            }
                        }
                    }
                    tbody { class: "divide-y divide-stone-100 text-stone-600",
                        for order in &data.orders {
                            tr { class: "hover:bg-stone-50",
                                td { class: "px-4 py-3 font-medium text-stone-900", "{order.number}" }
                                td { class: "px-4 py-3",
                                    time { datetime: iso_date_time(order.placed_at),
                                        {short_date_time(order.placed_at)}
                                    }
                                }
                                td { class: "px-4 py-3 text-stone-900", "{order.customer}" }
                                td { class: "px-4 py-3 text-stone-500", "{order.email}" }
                                td { class: "px-4 py-3", "{order.country}" }
                                td { class: "px-4 py-3 text-right tabular-nums", "{order.items}" }
                                td { class: "px-4 py-3",
                                    StatusBadge { status: order.status }
                                }
                                td { class: "px-4 py-3 text-right font-medium tabular-nums text-stone-900",
                                    "{Money(order.total_cents)}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
