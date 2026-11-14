/** Name of the cookie that holds the number of items in the cart. */
export const CART_COOKIE = "cart_count";

/** The largest cart count the cookie holds. */
export const CART_MAX = 999_999_999;

/** Parses a cart cookie value: 1 to 9 ASCII digits, anything else is an empty cart. */
export function parseCartCount(value: string | undefined): number {
  return value !== undefined && /^[0-9]{1,9}$/.test(value) ? Number(value) : 0;
}

/** The text of the cart badge, which caps at `99+`. */
export function cartBadgeText(count: number): string {
  return count > 99 ? "99+" : String(count);
}
