# Harbor Market: Next.js

The Harbor Market storefront on the Next.js App Router. Pages are React Server Components rendered on every request; the two interactive pieces are client components that hydrate in the browser.

## Stack

| Piece | Version |
|-------|---------|
| [Next.js](https://nextjs.org) App Router, built with Turbopack | 16.4.6 |
| React (the App Router uses the build vendored by Next.js) | 19.3.0 (vendored `19.3.0-canary-278794d7-20261002`) |
| Node.js, from the Nix shell | 24.21.0 |
| Tailwind CSS through `@tailwindcss/postcss` | 4.3.3, the shell's `tailwindcss` version |
| TypeScript | 5.9.3 |
| pnpm | 12.9.0 |

Dependencies are pinned by `pnpm-lock.yaml` and installed into `node_modules` here.

## Running

```sh
pnpm install --frozen-lockfile
pnpm build && cp -R .next/static .next/standalone/.next/
PORT=3000 BENCH_THREADS=4 node server.mjs      # data from ../../data unless BENCH_DATA_DIR is set
```

`pnpm dev` runs the development server.

## Layout

- `src/app/`: one folder per route. Pages (`page.tsx`), the root layout with the header, footer and cart cookie, `not-found.tsx` for every 404, `invalid-request/` for the 422 page, and route handlers (`route.ts`) for `/health` and the JSON API.
- `src/components/`: shared components, one per file. `cart.tsx`, `mobile-menu.tsx` and `purchase-form.tsx` are the only client components (`"use client"`).
- `src/lib/`: the dataset and its startup indexes (`store.ts`), listing validation (`listing.ts`), formatting, link builders and the cart cookie rule.
- `src/proxy.ts`: sends the 422 page for invalid listing queries (see below).
- `src/instrumentation.ts`: loads the dataset when each server process starts.
- `server.mjs`: the process launcher, the one piece that does not come from `create-next-app` or the Next.js docs.

## Threads and compression

- **Threads.** Node.js runs JavaScript on one thread, so a Next.js server uses one core. Real deployments scale out by process: PM2 cluster mode, several containers, or several instances behind a load balancer. `server.mjs` does what PM2 cluster mode does, in a few lines of `node:cluster`: with `BENCH_THREADS` unset or `0` it forks one worker per core (`os.availableParallelism()`), with `N` it forks `N` workers, and with `1` it runs the standalone server in its own process with no cluster at all. Each worker is an unmodified `.next/standalone/server.js`. Workers share the listening socket through the cluster primary, which hands out connections round-robin.
- **Compression.** Next.js gzips responses by default ([`compress`](https://nextjs.org/docs/app/api-reference/config/next-config-js/compress)). `next.config.ts` sets `compress: process.env.BENCH_COMPRESSION === "on"`, so it is genuinely off unless the variable is `on`, and then uses Next's built-in gzip with its defaults. The standalone server freezes `next.config.ts` at build time ([`output`](https://nextjs.org/docs/app/api-reference/config/next-config-js/output)); the runner sets `BENCH_COMPRESSION` for builds too, and `server.mjs` refuses to start when the build and the environment disagree (for example after `--skip-build` with a different setting).

## Judgment calls

- **Production server.** `output: "standalone"` and `node .next/standalone/server.js`, as the [self-hosting guide](https://nextjs.org/docs/app/guides/self-hosting) and the `output` docs describe, with `.next/static` copied next to it so it serves the assets with `Cache-Control: public, max-age=31536000, immutable`. `NODE_ENV=production` is set by `server.js` itself. Telemetry is off (`NEXT_TELEMETRY_DISABLED=1`). The production server writes no request logs.
- **Every route is dynamic.** The root layout reads the `cart_count` cookie with [`cookies()`](https://nextjs.org/docs/app/api-reference/functions/cookies), which opts every page into dynamic rendering. `next build` lists every route as `ƒ (Dynamic)`, including `/health` and the API routes; nothing is prerendered except Next's built-in `_global-error` page, and there are no `fetch` calls, so neither the full route cache nor the data cache is involved. Dynamic pages are sent with `Cache-Control: private, no-cache, no-store`.
- **No Cache Components.** `create-next-app` 16.4 turns on [`cacheComponents`](https://nextjs.org/docs/app/api-reference/config/next-config-js/cacheComponents) and `partialPrefetching`. Cache Components prerenders a static HTML shell for each route at build time and streams only the dynamic parts, which is exactly the static prerendering the benchmark rules forbid for benchmarked routes. The app uses the still supported [previous caching model](https://nextjs.org/docs/app/guides/caching-without-cache-components), where a route that reads cookies renders in full on every request.
- **Tailwind.** Installed as the Next.js [CSS guide](https://nextjs.org/docs/app/getting-started/css#tailwind-css) shows: `@tailwindcss/postcss` in `postcss.config.mjs` and `@import "tailwindcss"` in `globals.css`, pinned to 4.3.3. The import adds `source("..")` so Tailwind scans only `src/`, like the reference, instead of also picking up words from this README. (`create-next-app` 16.4 scaffolds the `@tailwindcss/turbopack` loader instead; both run the same Tailwind compiler.) Next.js runs the result through its own CSS minifier with its default browser targets, which adds color fallbacks, so the stylesheet is about 2 KB larger than the reference's with the same utility classes.
- **Data.** The dataset is loaded at startup by the [instrumentation hook](https://nextjs.org/docs/app/guides/instrumentation): `register()` in `src/instrumentation.ts` runs once in every worker process and must finish before the server handles requests, so `/health` only answers once the data is loaded. `getStore()` keeps the instance on `globalThis`, the usual Next.js pattern for process-wide singletons, so the instrumentation bundle, every route bundle and the proxy share it. Loading parses the JSON into typed records: dates become `Date` values (`createdOn`, `postedOn`, `placedAt`) and each product's category and related products are resolved to records. The original ISO strings are kept, and the detail API serializes exactly the JSON fields through `productJson()`. Formatting is per request, with cached `Intl.DateTimeFormat` instances for dates and cached `Intl.NumberFormat` instances for money and counts.
- **Default middleware and headers.** Nothing that Next.js sets up is removed: responses keep `X-Powered-By`, `Vary` and Next's `Cache-Control` headers.
- **Client components.** The SPEC's two interactive pieces are `MobileMenu` (owns the open state; the nav links and the cart pill are server-rendered and passed in as props) and `PurchaseForm` (the quantity stepper and add-to-cart). The cart badge lives in the header and is updated by the purchase form, so the count is shared through a React context: `CartProvider` wraps the layout with the count the server read from the cookie, and `CartBadge` is a tiny client component that reads it. This is the React way to share state between two client components in different parts of the tree; calling `router.refresh()` after writing the cookie would also work but costs a server round trip. The sold-out button has no behavior and stays a server component.
- **Links.** `next/link` everywhere a page links to another page, with the default `prefetch` (`"auto"`): in production, links that enter the viewport prefetch the route, and for dynamic routes like these that means only the part above the nearest `loading.js`, of which there is none ([docs](https://nextjs.org/docs/app/api-reference/components/link#prefetch)). The benchmark only measures document requests, so prefetching affects neither throughput nor the measured payload, only what a real browser would fetch after load. The footer's `/api/products` link is a plain `<a>` because it points at a route handler, not a page; `#main` and `#reviews` are plain in-page anchors.
- **Metadata.** Titles come from the [Metadata API](https://nextjs.org/docs/app/getting-started/metadata-and-og-images): a `%s | Harbor Market` template in the root layout, static `metadata` on fixed pages and `generateMetadata` on the listing and detail pages. The icon link is `icons: { icon: "data:," }`; Next.js adds the charset and viewport tags itself.
- **The 422 page.** A Next.js page can answer 200, or 404 through `notFound()`, but cannot choose another status, and a proxy rewrite keeps the status of the page it renders. For invalid `sort` or `page` values `src/proxy.ts` therefore renders `/invalid-request` with a request of its own and returns it with status 422. The proxy's `matcher` only matches listing URLs whose `sort` or `page` is invalid, so valid listing requests, including the benchmarked ones, never run it. Unknown categories and out-of-range pages are 404s from `notFound()` in the page, and the JSON API returns its errors directly from the route handler.
- **Repeated query parameters.** Next.js does not reject them; the page and the API use the first value. The proxy matcher looks at the last one, so `?sort=bad&sort=newest` gets a 404 page instead of the 422 page. The SPEC leaves this case unverified.
- **Product ids** are accepted only as plain decimal digits (`/products/01` is product 1, `/products/+1` is a 404).

## Size of what is shipped

The HTML of every page is about 2.6 times the reference's, because React Server Components inline the RSC payload, the serialized component tree that hydration and client-side navigation need, in `<script>` tags after the markup. The home and detail pages load about 460 KB of JavaScript (about 140 KB gzipped): React, React DOM, the Next.js client router and the three client components. Next.js also emits a `noModule` polyfill script (about 110 KB) that modern browsers skip.
