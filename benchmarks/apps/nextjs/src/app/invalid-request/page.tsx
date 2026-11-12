import type { Metadata } from "next";

import { ErrorContent, errorTitle } from "@/components/error-content";

export const metadata: Metadata = { title: errorTitle(422) };

/** The 422 page. `src/proxy.ts` rewrites invalid listing URLs here with a 422 status. */
export default function InvalidRequestPage() {
  return <ErrorContent status={422} />;
}
