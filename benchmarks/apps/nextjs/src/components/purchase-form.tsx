"use client";

import { useState } from "react";

import { CART_COOKIE, CART_MAX, parseCartCount } from "@/lib/cart";
import { useCart } from "./cart";
import { MinusIcon, PlusIcon } from "./icons";

const STEPPER_BUTTON =
  "rounded-full p-2.5 text-stone-700 hover:bg-stone-100 disabled:cursor-not-allowed disabled:opacity-30 disabled:hover:bg-transparent";

/** Reads the cart count from the cookie with the same rule as the server. */
function readCartCookie(): number {
  const match = document.cookie.match(/(?:^|;\s*)cart_count=([^;]*)/);
  return parseCartCount(match?.[1]);
}

/** The quantity stepper and add-to-cart button of an in-stock product. */
export function PurchaseForm({ max }: { max: number }) {
  const [quantity, setQuantity] = useState(1);
  const [status, setStatus] = useState("");
  const { setCount } = useCart();

  const step = (delta: number) =>
    setQuantity((quantity) => Math.min(max, Math.max(1, quantity + delta)));

  const addToCart = () => {
    const count = Math.min(CART_MAX, readCartCookie() + quantity);
    document.cookie = `${CART_COOKIE}=${count}; Path=/; Max-Age=2592000; SameSite=Lax`;
    setCount(count);
    setStatus(`Added ${quantity} to your cart.`);
  };

  return (
    <div className="mt-8">
      <div className="flex flex-col gap-3 sm:flex-row">
        <div className="flex items-center justify-between rounded-full border border-stone-300 bg-white p-1 sm:w-36">
          <button
            type="button"
            aria-label="Decrease quantity"
            disabled={quantity <= 1}
            onClick={() => step(-1)}
            className={STEPPER_BUTTON}
          >
            <MinusIcon />
          </button>
          <output
            aria-live="polite"
            aria-label="Quantity"
            className="w-10 text-center font-semibold tabular-nums"
          >
            {quantity}
          </output>
          <button
            type="button"
            aria-label="Increase quantity"
            disabled={quantity >= max}
            onClick={() => step(1)}
            className={STEPPER_BUTTON}
          >
            <PlusIcon />
          </button>
        </div>
        <button
          type="button"
          onClick={addToCart}
          className="flex-1 rounded-full bg-teal-700 px-8 py-3.5 text-base font-semibold text-white shadow-sm hover:bg-teal-800 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-teal-700"
        >
          Add to cart
        </button>
      </div>
      <p role="status" className="mt-3 min-h-5 text-sm font-medium text-teal-800">
        {status}
      </p>
    </div>
  );
}
