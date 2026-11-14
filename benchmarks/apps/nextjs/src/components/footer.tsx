import Link from "next/link";

import { LogoIcon } from "./icons";

const COLUMNS = [
  {
    title: "Shop",
    links: [
      { href: "/products", label: "All products" },
      { href: "/products?category=outdoor", label: "Outdoor & Camping" },
      { href: "/products?category=kitchen", label: "Kitchen & Dining" },
      { href: "/products?category=audio", label: "Audio" },
      { href: "/products?category=toys", label: "Toys & Games" },
    ],
  },
  {
    title: "Help",
    links: [
      { href: "/about#shipping", label: "Shipping & returns" },
      { href: "/about#warranty", label: "Warranty & repairs" },
      { href: "/about#contact", label: "Contact us" },
    ],
  },
  {
    title: "Company",
    links: [
      { href: "/about", label: "About us" },
      { href: "/admin/orders", label: "Order admin" },
      // A JSON route handler, not a page, so a plain link rather than a client-side navigation.
      { href: "/api/products", label: "Product API", api: true },
    ],
  },
];

/** The site footer with link columns. */
export function Footer() {
  return (
    <footer className="mt-24 bg-stone-900 text-stone-400">
      <div className="mx-auto max-w-7xl px-4 py-16 sm:px-6 lg:px-8">
        <div className="grid gap-12 md:grid-cols-5">
          <div className="md:col-span-2">
            <Link
              href="/"
              className="flex items-center gap-2.5 text-lg font-semibold tracking-tight text-white"
            >
              <LogoIcon />
              Harbor Market
            </Link>
            <p className="mt-4 max-w-xs text-sm leading-6">
              Well-made gear for every harbor day. Shipped from Portland, Oregon since 2016.
            </p>
          </div>
          {COLUMNS.map(({ title, links }) => (
            <div key={title}>
              <h2 className="text-sm font-semibold text-white">{title}</h2>
              <ul className="mt-4 space-y-3 text-sm">
                {links.map(({ href, label, api }) => (
                  <li key={href}>
                    {api ? (
                      <a href={href} className="hover:text-white">
                        {label}
                      </a>
                    ) : (
                      <Link href={href} className="hover:text-white">
                        {label}
                      </Link>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>
        <div className="mt-12 flex flex-col gap-2 border-t border-stone-800 pt-8 text-xs sm:flex-row sm:justify-between">
          <p>Copyright 2026 Harbor Market. All rights reserved.</p>
          <p>A demo storefront for comparing web frameworks.</p>
        </div>
      </div>
    </footer>
  );
}
