import Link from "next/link";

import type { Listing } from "@/lib/listing";
import { listingUrl } from "@/lib/urls";
import { ArrowLeftIcon, ArrowRightIcon } from "./icons";

const STEP =
  "flex items-center gap-2 rounded-full px-3 py-2 text-sm font-medium text-stone-700 hover:bg-stone-100";
const STEP_DISABLED =
  "flex items-center gap-2 rounded-full px-3 py-2 text-sm font-medium text-stone-300";
const PAGE =
  "flex size-10 items-center justify-center rounded-full text-sm font-medium text-stone-600 hover:bg-stone-100";
const PAGE_CURRENT =
  "flex size-10 items-center justify-center rounded-full bg-stone-900 text-sm font-semibold text-white";

/** Previous and next links with a link to every page of the listing. */
export function Pagination({ listing }: { listing: Listing }) {
  const url = (page: number) =>
    listingUrl({ category: listing.category?.slug, sort: listing.sort, page });
  const pages = Array.from({ length: listing.pageCount }, (_, index) => index + 1);
  return (
    <nav
      aria-label="Pagination"
      className="mt-12 flex items-center justify-between gap-4 border-t border-stone-200 pt-6"
    >
      {listing.page > 1 ? (
        <Link href={url(listing.page - 1)} className={STEP}>
          <ArrowLeftIcon />
          Previous
        </Link>
      ) : (
        <span aria-disabled="true" className={STEP_DISABLED}>
          <ArrowLeftIcon />
          Previous
        </span>
      )}
      <p className="text-sm text-stone-600 sm:hidden">
        Page {listing.page} of {listing.pageCount}
      </p>
      <ul className="hidden items-center gap-1 sm:flex">
        {pages.map((page) => (
          <li key={page}>
            {page === listing.page ? (
              <Link href={url(page)} aria-current="page" className={PAGE_CURRENT}>
                {page}
              </Link>
            ) : (
              <Link href={url(page)} className={PAGE}>
                {page}
              </Link>
            )}
          </li>
        ))}
      </ul>
      {listing.page < listing.pageCount ? (
        <Link href={url(listing.page + 1)} className={STEP}>
          Next
          <ArrowRightIcon />
        </Link>
      ) : (
        <span aria-disabled="true" className={STEP_DISABLED}>
          Next
          <ArrowRightIcon />
        </span>
      )}
    </nav>
  );
}
