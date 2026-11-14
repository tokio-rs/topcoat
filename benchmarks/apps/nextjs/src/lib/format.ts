// Formatting for the storefront. The formatters are created once and reused, as `Intl` recommends.

const money = new Intl.NumberFormat("en-US", { style: "currency", currency: "USD" });
const count = new Intl.NumberFormat("en-US");
const shortDate = new Intl.DateTimeFormat("en-US", {
  month: "short",
  day: "numeric",
  year: "numeric",
  timeZone: "UTC",
});
const time = new Intl.DateTimeFormat("en-US", {
  hour: "2-digit",
  minute: "2-digit",
  hourCycle: "h23",
  timeZone: "UTC",
});

/** An amount of money in cents, like `$1,234.50`. */
export function formatMoney(cents: number): string {
  return money.format(cents / 100);
}

/** An integer with thousands separators, like `1,204`. */
export function formatCount(value: number): string {
  return count.format(value);
}

/** A rating with one decimal, like `4.0`. */
export function formatRating(rating: number): string {
  return rating.toFixed(1);
}

/** A calendar date, like `Mar 4, 2026`. */
export function formatDate(date: Date): string {
  return shortDate.format(date);
}

/** A UTC timestamp, like `Mar 4, 2026 14:05`. */
export function formatDateTime(date: Date): string {
  return `${shortDate.format(date)} ${time.format(date)}`;
}

/** A date in ISO 8601 form, like `2026-03-04`, for `datetime` attributes. */
export function isoDate(date: Date): string {
  return date.toISOString().slice(0, 10);
}

/** A UTC timestamp in ISO 8601 form without seconds, like `2026-03-04T14:05Z`. */
export function isoDateTime(date: Date): string {
  return `${date.toISOString().slice(0, 16)}Z`;
}
