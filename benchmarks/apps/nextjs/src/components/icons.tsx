// Inline SVG icons. All are decorative and hidden from assistive technology.

/** The Harbor Market logo mark. */
export function LogoIcon() {
  return (
    <svg viewBox="0 0 32 32" aria-hidden="true" className="size-8">
      <rect width="32" height="32" rx="9" className="fill-teal-700" />
      <path
        d="M16 6v10M12 10h8M7 20c3 0 3-2.5 6-2.5s3 2.5 6 2.5 3-2.5 6-2.5M7 25c3 0 3-2.5 6-2.5s3 2.5 6 2.5 3-2.5 6-2.5"
        fill="none"
        stroke="white"
        strokeWidth="2"
        strokeLinecap="round"
      />
    </svg>
  );
}

/** A 24x24 stroked icon with a single path. */
function Outline({ className, d }: { className: string; d: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      className={className}
    >
      <path d={d} />
    </svg>
  );
}

export const BagIcon = () => <Outline className="size-5" d="M6 8h12l-1 12H7L6 8ZM9 8a3 3 0 0 1 6 0" />;
export const MenuIcon = () => <Outline className="size-6" d="M4 6h16M4 12h16M4 18h16" />;
export const ChevronRightIcon = () => <Outline className="size-4 text-stone-300" d="m9 6 6 6-6 6" />;
export const ArrowLeftIcon = () => <Outline className="size-4" d="M19 12H5m6-6-6 6 6 6" />;
export const ArrowRightIcon = () => <Outline className="size-4" d="M5 12h14m-6-6 6 6-6 6" />;
export const MinusIcon = () => <Outline className="size-4" d="M5 12h14" />;
export const PlusIcon = () => <Outline className="size-4" d="M12 5v14M5 12h14" />;
export const CheckIcon = () => <Outline className="size-5 text-teal-700" d="m5 12 5 5L20 7" />;
export const TruckIcon = () => (
  <Outline
    className="size-6"
    d="M3 6h11v10H3zM14 10h4l3 3v3h-7M7 19a2 2 0 1 0 0-4 2 2 0 0 0 0 4ZM17 19a2 2 0 1 0 0-4 2 2 0 0 0 0 4Z"
  />
);
export const RefreshIcon = () => (
  <Outline
    className="size-6"
    d="M4 12a8 8 0 0 1 14-5.3L20 9M20 4v5h-5M20 12a8 8 0 0 1-14 5.3L4 15M4 20v-5h5"
  />
);
export const ShieldIcon = () => (
  <Outline className="size-6" d="M12 3 5 6v6c0 4.5 3 7.5 7 9 4-1.5 7-4.5 7-9V6l-7-3Zm-3 9 2 2 4-4" />
);
