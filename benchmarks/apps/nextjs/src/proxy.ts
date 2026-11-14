import { type NextRequest, NextResponse } from "next/server";

import { resolveListing } from "@/lib/listing";
import { getStore } from "@/lib/store";

/**
 * Answers listing URLs with an invalid `sort` or `page` with the 422 page.
 *
 * A page can only answer 200 or, through `notFound()`, 404, and a rewrite keeps the status of the
 * page it renders. So the proxy renders the 422 page with a request of its own and passes it on
 * with the right status. The matcher keeps valid listing requests from running the proxy at all.
 */
export async function proxy(request: NextRequest) {
  const query = request.nextUrl.searchParams;
  const result = resolveListing(getStore(), {
    category: query.get("category"),
    sort: query.get("sort"),
    page: query.get("page"),
  });
  if (result.ok || result.error.status !== 422) return NextResponse.next();

  const page = await fetch(new URL("/invalid-request", request.url), {
    headers: { Cookie: request.headers.get("Cookie") ?? "" },
    cache: "no-store",
  });
  return new NextResponse(page.body, {
    status: 422,
    headers: { "Content-Type": page.headers.get("Content-Type") ?? "text/html; charset=utf-8" },
  });
}

export const config = {
  matcher: [
    {
      source: "/products",
      has: [
        { type: "query", key: "sort", value: "(?!(?:newest|price-asc|price-desc|rating)$).+" },
      ],
    },
    { source: "/products", has: [{ type: "query", key: "page", value: ".*[^0-9].*|0+" }] },
  ],
};
