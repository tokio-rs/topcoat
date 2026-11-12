use topcoat::{
    Result,
    context::Cx,
    metadata::Metadata,
    router::page,
    view::{View, view},
};

use crate::{
    components::status_badge::status_badge,
    data::store,
    format::{Count, Money, iso_date_time, short_date_time},
};

const COLUMNS: [&str; 8] = [
    "Order", "Placed", "Customer", "Email", "Country", "Items", "Status", "Total",
];

/// `GET /admin/orders`: summary cards and a table of every order, newest first.
#[page(with(Metadata::new().title("Orders | Harbor Market")))]
pub async fn orders(cx: &Cx) -> Result<impl View> {
    let store = store(cx);
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
    Ok(view! {
        <div class="mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8">
            <p class="text-sm font-semibold uppercase tracking-widest text-teal-700">
                "Admin"
            </p>
            <h1 class="mt-2 text-3xl font-semibold tracking-tight sm:text-4xl">
                "Orders"
            </h1>
            <p class="mt-2 text-stone-600">
                "All "
                (Count(summary.count as u64).to_string())
                " orders, newest first."
            </p>
            <dl class="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
                for (label, value, note) in stats {
                    <div class="rounded-2xl border border-stone-200 bg-white p-5">
                        <dt class="text-sm font-medium text-stone-500">(label)</dt>
                        <dd
                            class="mt-2 text-2xl font-semibold tracking-tight text-stone-900"
                        >
                            (value)
                        </dd>
                        <dd class="mt-1 text-xs text-stone-500">(note)</dd>
                    </div>
                }
            </dl>
            <div
                class="mt-8 overflow-x-auto rounded-2xl border border-stone-200 bg-white"
            >
                <table class="min-w-full whitespace-nowrap text-left text-sm">
                    <thead
                        class="border-b border-stone-200 bg-stone-50 text-xs font-semibold uppercase tracking-wide text-stone-500"
                    >
                        <tr>
                            for column in COLUMNS {
                                if column == "Items" || column == "Total" {
                                    <th scope="col" class="px-4 py-3 text-right">(column)</th>
                                } else {
                                    <th scope="col" class="px-4 py-3">(column)</th>
                                }
                            }
                        </tr>
                    </thead>
                    <tbody class="divide-y divide-stone-100 text-stone-600">
                        #[key(order.id)]
                        for order in store.orders() {
                            <tr class="hover:bg-stone-50">
                                <td class="px-4 py-3 font-medium text-stone-900">
                                    (&order.number)
                                </td>
                                <td class="px-4 py-3">
                                    <time datetime=(iso_date_time(order.placed_at))>
                                        (short_date_time(order.placed_at))
                                    </time>
                                </td>
                                <td class="px-4 py-3 text-stone-900">(&order.customer)</td>
                                <td class="px-4 py-3 text-stone-500">(&order.email)</td>
                                <td class="px-4 py-3">(&order.country)</td>
                                <td class="px-4 py-3 text-right tabular-nums">
                                    (order.items)
                                </td>
                                <td class="px-4 py-3">
                                    status_badge(status: order.status)
                                </td>
                                <td
                                    class="px-4 py-3 text-right font-medium tabular-nums text-stone-900"
                                >
                                    (Money(order.total_cents).to_string())
                                </td>
                            </tr>
                        }
                    </tbody>
                </table>
            </div>
        </div>
    })
}
