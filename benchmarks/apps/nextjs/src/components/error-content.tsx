import Link from "next/link";

const ERRORS = {
  404: {
    title: "Page not found",
    message: "Sorry, we couldn't find the page you're looking for.",
  },
  422: {
    title: "Invalid request",
    message:
      "Some of the filters in this link aren't valid. Try browsing from the catalog instead.",
  },
};

export function errorTitle(status: keyof typeof ERRORS): string {
  return ERRORS[status].title;
}

/** The body of an error page, rendered in the shared layout. */
export function ErrorContent({ status }: { status: keyof typeof ERRORS }) {
  const { title, message } = ERRORS[status];
  return (
    <div className="mx-auto max-w-2xl px-4 py-24 text-center sm:py-32">
      <p className="text-sm font-semibold text-teal-700">{status}</p>
      <h1 className="mt-3 text-4xl font-semibold tracking-tight text-stone-900">{title}</h1>
      <p className="mt-4 text-stone-600">{message}</p>
      <div className="mt-10 flex flex-wrap justify-center gap-4">
        <Link
          href="/"
          className="rounded-full bg-teal-700 px-6 py-3 text-sm font-semibold text-white hover:bg-teal-800"
        >
          Back to home
        </Link>
        <Link
          href="/products"
          className="rounded-full px-6 py-3 text-sm font-semibold text-stone-700 ring-1 ring-inset ring-stone-300 hover:bg-stone-100"
        >
          Browse products
        </Link>
      </div>
    </div>
  );
}
