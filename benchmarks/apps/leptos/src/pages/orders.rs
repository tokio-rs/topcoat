use std::sync::Arc;

use leptos::{either::Either, prelude::*};
use serde::{Deserialize, Serialize};

use super::ErrorPage;
use crate::{
    components::{PageTitle, StatusBadge},
    format::{Count, IsoDateTime, Money, ShortDateTime},
    model::{Order, OrderSummary, PageError},
};

const COLUMNS: [&str; 8] = [
    "Order", "Placed", "Customer", "Email", "Country", "Items", "Status", "Total",
];

/// The data of the orders page.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrdersData {
    summary: OrderSummary,
    orders: Arc<[Order]>,
}

/// Loads every order, newest first, with the order totals.
#[server]
pub async fn load_orders() -> Result<OrdersData, PageError> {
    let store = expect_context::<Arc<crate::store::Store>>();
    Ok(OrdersData {
        summary: store.order_summary().clone(),
        orders: store.orders().clone(),
    })
}

/// `GET /admin/orders`: summary cards and a table of every order, newest first.
#[component]
pub fn OrdersPage() -> impl IntoView {
    let data = Resource::new(|| (), |()| load_orders());
    view! {
        <Suspense>
            {move || Suspend::new(async move {
                match data.await {
                    Ok(data) => Either::Left(view! { <Orders data /> }),
                    Err(error) => Either::Right(view! { <ErrorPage error /> }),
                }
            })}
        </Suspense>
    }
}

#[component]
fn Orders(data: OrdersData) -> impl IntoView {
    let summary = data.summary;
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
    view! {
        <PageTitle title="Orders" />
        <div class="mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8">
            <p class="text-sm font-semibold uppercase tracking-widest text-teal-700">"Admin"</p>
            <h1 class="mt-2 text-3xl font-semibold tracking-tight sm:text-4xl">"Orders"</h1>
            <p class="mt-2 text-stone-600">
                {format!("All {} orders, newest first.", Count(summary.count as u64))}
            </p>
            <dl class="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                {stats
                    .map(|(label, value, note)| {
                        view! {
                            <div class="rounded-2xl border border-stone-200 bg-white p-5">
                                <dt class="text-sm font-medium text-stone-500">{label}</dt>
                                <dd class="mt-2 text-2xl font-semibold tracking-tight text-stone-900">
                                    {value}
                                </dd>
                                <dd class="mt-1 text-xs text-stone-500">{note}</dd>
                            </div>
                        }
                    })}
            </dl>
            <div class="mt-8 overflow-x-auto rounded-2xl border border-stone-200 bg-white">
                <table class="min-w-full whitespace-nowrap text-left text-sm">
                    <thead class="border-b border-stone-200 bg-stone-50 text-xs font-semibold uppercase tracking-wide text-stone-500">
                        <tr>
                            {COLUMNS
                                .map(|column| {
                                    let numeric = column == "Items" || column == "Total";
                                    view! {
                                        <th
                                            scope="col"
                                            class=if numeric { "px-4 py-3 text-right" } else { "px-4 py-3" }
                                        >
                                            {column}
                                        </th>
                                    }
                                })}
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-stone-100 text-stone-600">
                        {data
                            .orders
                            .iter()
                            .map(|order| {
                                view! {
                                    <tr class="hover:bg-stone-50">
                                        <td class="px-4 py-3 font-medium text-stone-900">
                                            {order.number.clone()}
                                        </td>
                                        <td class="px-4 py-3">
                                            <time datetime=IsoDateTime(order.placed_at)
                                                .to_string()>{ShortDateTime(order.placed_at).to_string()}</time>
                                        </td>
                                        <td class="px-4 py-3 text-stone-900">{order.customer.clone()}</td>
                                        <td class="px-4 py-3 text-stone-500">{order.email.clone()}</td>
                                        <td class="px-4 py-3">{order.country.clone()}</td>
                                        <td class="px-4 py-3 text-right tabular-nums">{order.items}</td>
                                        <td class="px-4 py-3">
                                            <StatusBadge status=order.status />
                                        </td>
                                        <td class="px-4 py-3 text-right font-medium tabular-nums text-stone-900">
                                            {Money(order.total_cents).to_string()}
                                        </td>
                                    </tr>
                                }
                            })
                            .collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}
