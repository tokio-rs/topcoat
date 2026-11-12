import Link from "next/link";

import { ArrowRightIcon, RefreshIcon, ShieldIcon, TruckIcon } from "@/components/icons";
import { ProductCard } from "@/components/product-card";
import { formatCount } from "@/lib/format";
import { getStore } from "@/lib/store";
import { listingUrl } from "@/lib/urls";

const PERKS = [
  {
    Icon: TruckIcon,
    title: "Free shipping over $75",
    text: "Orders ship within one business day from Portland.",
  },
  { Icon: RefreshIcon, title: "30-day returns", text: "Changed your mind? Send it back, on us." },
  {
    Icon: ShieldIcon,
    title: "Repair, don't replace",
    text: "Spare parts and repairs for as long as we sell it.",
  },
];

/** `GET /`: hero, store perks, category links and featured products. */
export default function HomePage() {
  const store = getStore();
  return (
    <>
      <section className="relative overflow-hidden bg-teal-900 text-white">
        <div className="mx-auto grid max-w-7xl items-center gap-12 px-4 py-20 sm:px-6 lg:grid-cols-2 lg:px-8 lg:py-28">
          <div>
            <p className="text-sm font-semibold uppercase tracking-widest text-teal-200">
              New season, same harbor
            </p>
            <h1 className="mt-4 text-4xl font-semibold tracking-tight text-balance sm:text-5xl lg:text-6xl">
              Well-made gear for every harbor day.
            </h1>
            <p className="mt-6 max-w-xl text-lg leading-8 text-teal-100">
              Tents, knives, boots and blankets from small workshops we know by name. Built to last,
              easy to repair and shipped free over $75.
            </p>
            <div className="mt-10 flex flex-wrap gap-4">
              <Link
                href="/products"
                className="rounded-full bg-white px-6 py-3 text-sm font-semibold text-teal-900 shadow-sm hover:bg-teal-50"
              >
                Shop all products
              </Link>
              <Link
                href={listingUrl({ category: "outdoor" })}
                className="rounded-full px-6 py-3 text-sm font-semibold text-white ring-1 ring-inset ring-white/40 hover:bg-white/10"
              >
                Browse outdoor gear
              </Link>
            </div>
          </div>
          <div aria-hidden="true" className="relative hidden aspect-[4/3] lg:block">
            <div className="absolute left-[8%] top-[10%] aspect-square w-[46%] -rotate-6 rounded-[28%] bg-amber-200 shadow-2xl shadow-black/30" />
            <div className="absolute right-[6%] top-[4%] aspect-square w-[34%] rotate-12 rounded-full bg-sky-400 shadow-2xl shadow-black/30" />
            <div className="absolute bottom-[2%] right-[18%] aspect-square w-[40%] rotate-6 rounded-[28%] bg-orange-700 shadow-2xl shadow-black/30" />
          </div>
        </div>
      </section>
      <section aria-label="Why Harbor Market" className="border-b border-stone-200 bg-white">
        <ul className="mx-auto grid max-w-7xl gap-8 px-4 py-10 sm:grid-cols-3 sm:px-6 lg:px-8">
          {PERKS.map(({ Icon, title, text }) => (
            <li key={title} className="flex gap-4">
              <span className="flex size-11 shrink-0 items-center justify-center rounded-full bg-teal-50 text-teal-700">
                <Icon />
              </span>
              <div>
                <p className="font-semibold text-stone-900">{title}</p>
                <p className="mt-1 text-sm text-stone-600">{text}</p>
              </div>
            </li>
          ))}
        </ul>
      </section>
      <section
        aria-labelledby="categories-heading"
        className="mx-auto max-w-7xl px-4 pt-16 sm:px-6 lg:px-8"
      >
        <h2 id="categories-heading" className="text-2xl font-semibold tracking-tight">
          Shop by category
        </h2>
        <ul className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {store.categories.map((category) => (
            <li key={category.slug}>
              <Link
                href={listingUrl({ category: category.slug })}
                className="group block h-full rounded-2xl border border-stone-200 bg-white p-5 transition hover:border-teal-600 hover:shadow-md"
              >
                <h3 className="font-semibold text-stone-900 group-hover:text-teal-800">
                  {category.name}
                </h3>
                <p className="mt-1 line-clamp-2 text-sm text-stone-600">{category.description}</p>
                <p className="mt-4 text-xs font-medium text-stone-500">
                  {formatCount(category.productCount)} products
                </p>
              </Link>
            </li>
          ))}
        </ul>
      </section>
      <section
        aria-labelledby="featured-heading"
        className="mx-auto max-w-7xl px-4 pt-20 sm:px-6 lg:px-8"
      >
        <div className="flex items-end justify-between gap-4">
          <h2 id="featured-heading" className="text-2xl font-semibold tracking-tight">
            Featured products
          </h2>
          <Link
            href="/products"
            className="flex items-center gap-1.5 text-sm font-semibold text-teal-700 hover:text-teal-900"
          >
            View all
            <ArrowRightIcon />
          </Link>
        </div>
        <div className="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 lg:grid-cols-4">
          {store.featured.map((product) => (
            <ProductCard key={product.id} product={product} />
          ))}
        </div>
      </section>
    </>
  );
}
