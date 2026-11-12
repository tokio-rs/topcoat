use topcoat::{
    Result,
    context::Cx,
    view::{View, class, component, view},
};

use crate::data::OrderStatus;

/// A colored pill showing an order's status.
#[component]
pub async fn status_badge(cx: &Cx, status: OrderStatus) -> Result<impl View> {
    let colors = match status {
        OrderStatus::Pending => class!("bg-amber-50 text-amber-800 ring-amber-600/20"),
        OrderStatus::Paid => class!("bg-sky-50 text-sky-800 ring-sky-600/20"),
        OrderStatus::Shipped => class!("bg-indigo-50 text-indigo-700 ring-indigo-600/20"),
        OrderStatus::Delivered => class!("bg-emerald-50 text-emerald-800 ring-emerald-600/20"),
        OrderStatus::Refunded => class!("bg-stone-100 text-stone-700 ring-stone-500/20"),
        OrderStatus::Cancelled => class!("bg-rose-50 text-rose-700 ring-rose-600/20"),
    };
    Ok(view! {
        <span
            class=(class!(
                "inline-flex rounded-full px-2 py-0.5 text-xs font-medium ring-1 ring-inset",
                colors,
            ))
        >
            (status.label())
        </span>
    })
}
