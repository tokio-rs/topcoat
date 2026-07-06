import type { Review } from "../lib/catalog";
import { RatingStars } from "./rating-stars";

export function ReviewList({ reviews }: { reviews: Review[] }) {
  return (
    <div className="mt-6 space-y-6">
      {reviews.map((review, index) => (
        <article key={index} className="rounded-xl border border-slate-200 bg-white p-6">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="font-semibold text-slate-900">{review.author}</p>
            <p className="text-xs text-slate-400">{review.date}</p>
          </div>
          <div className="mt-2 flex items-center gap-2">
            <RatingStars tenths={review.rating_tenths} size="h-4 w-4" />
            <p className="text-sm font-medium text-slate-700">{review.title}</p>
          </div>
          <p className="mt-3 text-sm text-slate-600">{review.body}</p>
        </article>
      ))}
    </div>
  );
}
