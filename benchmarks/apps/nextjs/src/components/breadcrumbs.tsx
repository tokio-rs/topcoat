import Link from "next/link";

import { ChevronRightIcon } from "./icons";

/** A breadcrumb trail of links ending in the current page. */
export function Breadcrumbs({
  trail,
  current,
}: {
  trail: { label: string; href: string }[];
  current: string;
}) {
  return (
    <nav aria-label="Breadcrumb" className="text-sm">
      <ol className="flex flex-wrap items-center gap-1.5 text-stone-500">
        {trail.map(({ label, href }) => (
          <li key={href} className="flex items-center gap-1.5">
            <Link href={href} className="hover:text-stone-900">
              {label}
            </Link>
            <ChevronRightIcon />
          </li>
        ))}
        <li>
          <span aria-current="page" className="font-medium text-stone-900">
            {current}
          </span>
        </li>
      </ol>
    </nav>
  );
}
