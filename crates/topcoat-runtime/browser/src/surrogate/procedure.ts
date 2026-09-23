import type { Context } from "../expression/context";
import { dehydrate } from "../expression/dehydrate";
import { Future } from "./future";

/** A procedure that compiled expressions can call on the server. */
export class Procedure<A extends unknown[] = unknown[], R = unknown> {
	constructor(
		private readonly cx: Context,
		private readonly id: string,
	) {}

	/**
	 * Calls the procedure. The request starts when the returned future is
	 * awaited, and a failed response rejects it.
	 */
	call(...args: A): Future<R> {
		return new Future(async () => {
			const response = await fetch(
				`/_topcoat/runtime/procedures/${encodeURIComponent(this.id)}`,
				{
					method: "POST",
					headers: { "Content-Type": "application/json" },
					body: JSON.stringify(args.map(dehydrate)),
				},
			);
			if (!response.ok) {
				throw new Error(
					`Procedure call failed: ${response.status} ${response.statusText}`,
				);
			}

			return this.cx.hydrate(await response.json()) as R;
		});
	}

	dehydrate(): { t: "Procedure"; id: string } {
		return { t: "Procedure", id: this.id };
	}
}
