const STAR_PATH = "M10 1l3 6 6 1-4 4 1 6-6-3-6 3 1-6-4-4 6-1z";

const CLASSES = {
  small: { filled: "size-4 fill-amber-400", empty: "size-4 fill-stone-300" },
  large: { filled: "size-5 fill-amber-400", empty: "size-5 fill-stone-300" },
};

/** The number of filled stars for a rating in tenths: the rating rounded half up. */
export function filledStars(ratingTenths: number): number {
  return Math.floor((ratingTenths + 5) / 10);
}

/**
 * A row of five stars with `filled` of them highlighted.
 *
 * The row is a single image to assistive technology, labelled with `rating`.
 */
export function Stars({
  filled,
  rating,
  size,
}: {
  filled: number;
  rating: string | number;
  size: "small" | "large";
}) {
  const classes = CLASSES[size];
  return (
    <span className="flex items-center gap-0.5" role="img" aria-label={`Rated ${rating} out of 5`}>
      {[0, 1, 2, 3, 4].map((star) => (
        <svg key={star} viewBox="0 0 20 20" className={star < filled ? classes.filled : classes.empty}>
          <path d={STAR_PATH} />
        </svg>
      ))}
    </span>
  );
}
