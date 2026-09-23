import type { Context } from "../expression/context";
import { dehydrate } from "../expression/dehydrate";
import { Future } from "./future";

export class Procedure<A extends unknown[] = unknown[], R = unknown> {
	constructor(
		private readonly cx: Context,
		/** The path of the procedure's endpoint, where calls are posted. */
		private readonly path: string,
	) {}

	call(...args: A): Future<R> {
		return new Future(async () => {
			const response = await fetch(this.path, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify(args.map(dehydrate)),
			});
			if (!response.ok) {
				throw new Error(
					`Procedure call failed: ${response.status} ${response.statusText}`,
				);
			}

			return this.cx.hydrate(await response.json()) as R;
		});
	}

	dehydrate(): { t: "Procedure"; path: string } {
		return { t: "Procedure", path: this.path };
	}
}
