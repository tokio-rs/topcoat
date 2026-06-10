import type { Context } from "../context";
import { Future } from "./future";

export class Action<A extends unknown[] = unknown[], R = unknown> {
	constructor(
		private readonly cx: Context,
		private readonly id: string,
	) {}

	call(...args: A): Future<R> {
		return new Future(async () => {
			const response = await fetch(
				`/_topcoat/actions/${encodeURIComponent(this.id)}`,
				{
					method: "POST",
					headers: { "Content-Type": "application/json" },
					body: JSON.stringify(args),
				},
			);
			if (!response.ok) {
				throw new Error(
					`Action call failed: ${response.status} ${response.statusText}`,
				);
			}

			return this.cx.s(await response.json()) as R;
		});
	}

	toJSON(): { t: "Action"; id: string } {
		return { t: "Action", id: this.id };
	}
}
