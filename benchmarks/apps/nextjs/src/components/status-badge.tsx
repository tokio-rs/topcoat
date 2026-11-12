import type { OrderStatus } from "@/lib/store";

const STATUSES: Record<OrderStatus, { label: string; colors: string }> = {
  pending: { label: "Pending", colors: "bg-amber-50 text-amber-800 ring-amber-600/20" },
  paid: { label: "Paid", colors: "bg-sky-50 text-sky-800 ring-sky-600/20" },
  shipped: { label: "Shipped", colors: "bg-indigo-50 text-indigo-700 ring-indigo-600/20" },
  delivered: { label: "Delivered", colors: "bg-emerald-50 text-emerald-800 ring-emerald-600/20" },
  refunded: { label: "Refunded", colors: "bg-stone-100 text-stone-700 ring-stone-500/20" },
  cancelled: { label: "Cancelled", colors: "bg-rose-50 text-rose-700 ring-rose-600/20" },
};

/** A colored pill showing an order's status. */
export function StatusBadge({ status }: { status: OrderStatus }) {
  const { label, colors } = STATUSES[status];
  return (
    <span
      className={`inline-flex rounded-full px-2 py-0.5 text-xs font-medium ring-1 ring-inset ${colors}`}
    >
      {label}
    </span>
  );
}
