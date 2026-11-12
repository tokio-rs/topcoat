import type { Metadata } from "next";

import { StatusBadge } from "@/components/status-badge";
import { formatCount, formatDateTime, formatMoney, isoDateTime } from "@/lib/format";
import { getStore } from "@/lib/store";

export const metadata: Metadata = { title: "Orders" };

const COLUMNS = ["Order", "Placed", "Customer", "Email", "Country", "Items", "Status", "Total"];
const RIGHT_ALIGNED = new Set(["Items", "Total"]);

/** `GET /admin/orders`: summary cards and a table of every order, newest first. */
export default function OrdersPage() {
  const { orders, orderSummary: summary } = getStore();
  const stats = [
    { label: "Orders", value: formatCount(summary.count), note: "All time" },
    {
      label: "Revenue",
      value: formatMoney(summary.revenueCents),
      note: "Paid, shipped and delivered",
    },
    {
      label: "Average order",
      value: formatMoney(summary.averageCents),
      note: "Across revenue orders",
    },
    { label: "Refunded", value: formatMoney(summary.refundedCents), note: "Returned to customers" },
  ];
  return (
    <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8">
      <p className="text-sm font-semibold uppercase tracking-widest text-teal-700">Admin</p>
      <h1 className="mt-2 text-3xl font-semibold tracking-tight sm:text-4xl">Orders</h1>
      <p className="mt-2 text-stone-600">All {formatCount(summary.count)} orders, newest first.</p>
      <dl className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {stats.map(({ label, value, note }) => (
          <div key={label} className="rounded-2xl border border-stone-200 bg-white p-5">
            <dt className="text-sm font-medium text-stone-500">{label}</dt>
            <dd className="mt-2 text-2xl font-semibold tracking-tight text-stone-900">{value}</dd>
            <dd className="mt-1 text-xs text-stone-500">{note}</dd>
          </div>
        ))}
      </dl>
      <div className="mt-8 overflow-x-auto rounded-2xl border border-stone-200 bg-white">
        <table className="min-w-full whitespace-nowrap text-left text-sm">
          <thead className="border-b border-stone-200 bg-stone-50 text-xs font-semibold uppercase tracking-wide text-stone-500">
            <tr>
              {COLUMNS.map((column) =>
                RIGHT_ALIGNED.has(column) ? (
                  <th key={column} scope="col" className="px-4 py-3 text-right">
                    {column}
                  </th>
                ) : (
                  <th key={column} scope="col" className="px-4 py-3">
                    {column}
                  </th>
                ),
              )}
            </tr>
          </thead>
          <tbody className="divide-y divide-stone-100 text-stone-600">
            {orders.map((order) => (
              <tr key={order.id} className="hover:bg-stone-50">
                <td className="px-4 py-3 font-medium text-stone-900">{order.number}</td>
                <td className="px-4 py-3">
                  <time dateTime={isoDateTime(order.placedAt)}>
                    {formatDateTime(order.placedAt)}
                  </time>
                </td>
                <td className="px-4 py-3 text-stone-900">{order.customer}</td>
                <td className="px-4 py-3 text-stone-500">{order.email}</td>
                <td className="px-4 py-3">{order.country}</td>
                <td className="px-4 py-3 text-right tabular-nums">{order.items}</td>
                <td className="px-4 py-3">
                  <StatusBadge status={order.status} />
                </td>
                <td className="px-4 py-3 text-right font-medium tabular-nums text-stone-900">
                  {formatMoney(order.total_cents)}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
