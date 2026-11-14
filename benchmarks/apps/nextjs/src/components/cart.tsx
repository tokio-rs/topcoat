"use client";

import { createContext, type Dispatch, type ReactNode, useContext, useState } from "react";

import { cartBadgeText } from "@/lib/cart";

const CartContext = createContext<{ count: number; setCount: Dispatch<number> } | null>(null);

/** Holds the cart count, starting from the value of the cookie the server read. */
export function CartProvider({
  initialCount,
  children,
}: {
  initialCount: number;
  children: ReactNode;
}) {
  const [count, setCount] = useState(initialCount);
  return <CartContext value={{ count, setCount }}>{children}</CartContext>;
}

export function useCart() {
  const cart = useContext(CartContext);
  if (!cart) throw new Error("useCart must be used inside a CartProvider");
  return cart;
}

/** The number in the header's cart pill. */
export function CartBadge() {
  const { count } = useCart();
  return (
    <span className="min-w-6 rounded-full bg-teal-700 px-2 py-0.5 text-center text-xs font-semibold text-white">
      {cartBadgeText(count)}
    </span>
  );
}
