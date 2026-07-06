import { filledStars } from "../lib/catalog";

// The heroicons solid star, shared verbatim by every benchmark app.
const STAR_PATH =
  "M9.049 2.927c.3-.921 1.603-.921 1.902 0l1.07 3.292a1 1 0 00.95.69h3.462c.969 0 1.371 1.24.588 1.81l-2.8 2.034a1 1 0 00-.364 1.118l1.07 3.292c.3.921-.755 1.688-1.539 1.118l-2.8-2.034a1 1 0 00-1.176 0l-2.8 2.034c-.783.57-1.838-.197-1.539-1.118l1.07-3.292a1 1 0 00-.363-1.118l-2.8-2.034c-.784-.57-.381-1.81.588-1.81h3.461a1 1 0 00.951-.69l1.07-3.292z";

export function RatingStars({ tenths, size }: { tenths: number; size: string }) {
  const filled = filledStars(tenths);

  return (
    <div className="flex">
      {[0, 1, 2, 3, 4].map((index) => (
        <svg
          key={index}
          viewBox="0 0 20 20"
          fill="currentColor"
          aria-hidden="true"
          className={`${size} ${index < filled ? "text-amber-400" : "text-slate-200"}`}
        >
          <path d={STAR_PATH}></path>
        </svg>
      ))}
    </div>
  );
}
