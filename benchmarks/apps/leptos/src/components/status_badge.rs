use leptos::prelude::*;

use crate::model::OrderStatus;

/// A colored pill showing an order's status.
#[component]
pub fn StatusBadge(status: OrderStatus) -> impl IntoView {
    let colors = match status {
        OrderStatus::Pending => "bg-amber-50 text-amber-800 ring-amber-600/20",
        OrderStatus::Paid => "bg-sky-50 text-sky-800 ring-sky-600/20",
        OrderStatus::Shipped => "bg-indigo-50 text-indigo-700 ring-indigo-600/20",
        OrderStatus::Delivered => "bg-emerald-50 text-emerald-800 ring-emerald-600/20",
        OrderStatus::Refunded => "bg-stone-100 text-stone-700 ring-stone-500/20",
        OrderStatus::Cancelled => "bg-rose-50 text-rose-700 ring-rose-600/20",
    };
    view! {
        <span class=format!(
            "inline-flex rounded-full px-2 py-0.5 text-xs font-medium ring-1 ring-inset {colors}",
        )>{status.label()}</span>
    }
}
