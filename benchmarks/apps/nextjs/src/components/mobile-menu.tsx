"use client";

import { type ReactNode, useState } from "react";

import { MenuIcon } from "./icons";

const NAV = "order-last hidden w-full md:order-none md:block md:w-auto";
const NAV_OPEN = "order-last w-full md:order-none md:block md:w-auto";

/**
 * The main navigation and the button that shows it on narrow screens.
 *
 * `links` and `actions` are rendered on the server; this component only owns the open state.
 */
export function MobileMenu({ links, actions }: { links: ReactNode; actions: ReactNode }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <nav id="site-nav" aria-label="Main" className={open ? NAV_OPEN : NAV}>
        {links}
      </nav>
      <div className="ml-auto flex items-center gap-2">
        {actions}
        <button
          type="button"
          aria-controls="site-nav"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
          className="rounded-md p-2 text-stone-600 hover:bg-stone-100 md:hidden"
        >
          <span className="sr-only">Toggle menu</span>
          <MenuIcon />
        </button>
      </div>
    </>
  );
}
