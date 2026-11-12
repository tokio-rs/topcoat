import type { Metadata } from "next";

import { ErrorContent, errorTitle } from "@/components/error-content";

export const metadata: Metadata = { title: errorTitle(404) };

/** The 404 page for unknown URLs and for `notFound()` calls. */
export default function NotFound() {
  return <ErrorContent status={404} />;
}
