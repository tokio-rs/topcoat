import type { Metadata } from "next";

export const metadata: Metadata = { title: "About" };

const FACTS = [
  { label: "Founded", value: "2016" },
  { label: "Workshops", value: "38" },
  { label: "Repairs last year", value: "2,140" },
  { label: "Orders shipped", value: "96,000+" },
];

/** `GET /about`: a small, static page in the shared layout. */
export default function AboutPage() {
  return (
    <div className="mx-auto max-w-3xl px-4 py-16 sm:px-6 lg:px-8">
      <p className="text-sm font-semibold uppercase tracking-widest text-teal-700">Our story</p>
      <h1 className="mt-3 text-4xl font-semibold tracking-tight text-balance">About Harbor Market</h1>
      <p className="mt-6 text-lg leading-8 text-stone-600">
        Harbor Market started in 2016 as a weekend stall at the Portland waterfront, selling camp
        mugs and wool blankets to people on their way to the coast. Today we work with small
        workshops around the world to make gear that is built to last and easy to repair.
      </p>
      <dl className="mt-12 grid grid-cols-2 gap-4 sm:grid-cols-4">
        {FACTS.map(({ label, value }) => (
          <div key={label} className="rounded-2xl border border-stone-200 bg-white p-5">
            <dt className="text-sm text-stone-500">{label}</dt>
            <dd className="mt-2 text-2xl font-semibold tracking-tight text-stone-900">{value}</dd>
          </div>
        ))}
      </dl>
      <div className="mt-16 space-y-12 text-stone-600">
        <section id="shipping">
          <h2 className="text-2xl font-semibold tracking-tight text-stone-900">Shipping &amp; returns</h2>
          <p className="mt-4 leading-7">
            Orders placed before 2 p.m. Pacific ship the same business day. Shipping is free on
            orders over $75; below that, it is a flat $7.95 anywhere in the contiguous United
            States.
          </p>
          <p className="mt-4 leading-7">
            If something isn&apos;t right, send it back within 30 days for a full refund. Return
            shipping is on us, and we will never charge a restocking fee.
          </p>
        </section>
        <section id="warranty">
          <h2 className="text-2xl font-semibold tracking-tight text-stone-900">Warranty &amp; repairs</h2>
          <p className="mt-4 leading-7">
            Everything we sell is covered against defects in materials and workmanship for at least
            one year, and most products for much longer. When something wears out, we would rather
            fix it than replace it.
          </p>
          <ul className="mt-4 list-disc space-y-2 pl-5">
            <li>Free repairs for the full warranty period.</li>
            <li>Spare parts at cost for as long as we sell a product.</li>
            <li>Repair guides for common fixes, written by our workshop team.</li>
          </ul>
        </section>
        <section id="contact">
          <h2 className="text-2xl font-semibold tracking-tight text-stone-900">Contact us</h2>
          <p className="mt-4 leading-7">
            Our support team answers every message within one business day, Monday to Friday, 8 a.m.
            to 6 p.m. Pacific.
          </p>
          <dl className="mt-6 grid gap-4 sm:grid-cols-2">
            <div className="rounded-2xl bg-white p-5 ring-1 ring-stone-200">
              <dt className="text-sm font-semibold text-stone-900">Email</dt>
              <dd className="mt-1">help@harbor-market.example</dd>
            </div>
            <div className="rounded-2xl bg-white p-5 ring-1 ring-stone-200">
              <dt className="text-sm font-semibold text-stone-900">Visit</dt>
              <dd className="mt-1">1200 NW Front Ave, Portland, OR 97209</dd>
            </div>
          </dl>
        </section>
      </div>
    </div>
  );
}
