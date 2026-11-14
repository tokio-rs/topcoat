import Link from "next/link";

import { CartBadge } from "./cart";
import { BagIcon, LogoIcon } from "./icons";
import { MobileMenu } from "./mobile-menu";

const NAV_LINKS = [
  { href: "/products", label: "All products" },
  { href: "/products?category=outdoor", label: "Outdoor" },
  { href: "/products?category=kitchen", label: "Kitchen" },
  { href: "/products?category=electronics", label: "Electronics" },
  { href: "/about", label: "About" },
];

/** The announcement bar and site header with navigation, cart badge and mobile menu toggle. */
export function Header() {
  return (
    <>
      <div className="bg-teal-900 px-4 py-2 text-center text-xs font-medium text-teal-50 sm:text-sm">
        Free shipping on orders over $75 and free 30-day returns
      </div>
      <header className="sticky top-0 z-20 border-b border-stone-200 bg-white/95 backdrop-blur">
        <div className="mx-auto flex max-w-7xl flex-wrap items-center gap-x-10 gap-y-3 px-4 py-3 sm:px-6 lg:px-8">
          <Link
            href="/"
            className="flex items-center gap-2.5 text-lg font-semibold tracking-tight text-stone-900"
          >
            <LogoIcon />
            Harbor Market
          </Link>
          <MobileMenu
            links={
              <ul className="flex flex-col gap-1 border-t border-stone-100 pt-3 md:flex-row md:gap-8 md:border-0 md:pt-0">
                {NAV_LINKS.map(({ href, label }) => (
                  <li key={href}>
                    <Link
                      href={href}
                      className="block rounded-md px-2 py-2 text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900 md:p-0 md:hover:bg-transparent"
                    >
                      {label}
                    </Link>
                  </li>
                ))}
              </ul>
            }
            actions={
              <div className="flex items-center gap-2 rounded-full border border-stone-200 bg-white py-1.5 pl-3 pr-1.5 text-sm font-medium text-stone-700">
                <BagIcon />
                <span className="hidden sm:inline">Cart</span>
                <CartBadge />
              </div>
            }
          />
        </div>
      </header>
    </>
  );
}
