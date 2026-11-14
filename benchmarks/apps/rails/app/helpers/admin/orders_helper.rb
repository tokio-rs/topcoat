module Admin::OrdersHelper
  STATUS_BADGES = {
    "pending" => [ "Pending", "bg-amber-50 text-amber-800 ring-amber-600/20" ],
    "paid" => [ "Paid", "bg-sky-50 text-sky-800 ring-sky-600/20" ],
    "shipped" => [ "Shipped", "bg-indigo-50 text-indigo-700 ring-indigo-600/20" ],
    "delivered" => [ "Delivered", "bg-emerald-50 text-emerald-800 ring-emerald-600/20" ],
    "refunded" => [ "Refunded", "bg-stone-100 text-stone-700 ring-stone-500/20" ],
    "cancelled" => [ "Cancelled", "bg-rose-50 text-rose-700 ring-rose-600/20" ]
  }.freeze

  # A colored pill with the order's status.
  def order_status_badge(order)
    label, colors = STATUS_BADGES.fetch(order.status)
    tag.span label, class: "inline-flex rounded-full px-2 py-0.5 text-xs font-medium ring-1 ring-inset #{colors}"
  end
end
