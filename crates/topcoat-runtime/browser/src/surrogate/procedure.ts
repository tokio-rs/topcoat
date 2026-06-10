import type { Context } from "../context";
import { Future } from "./future";

export class Procedure<A extends unknown[] = unknown[], R = unknown> {
	constructor(
		private readonly cx: Context,
		private readonly id: string,
	) {}

	call(...args: A): Future<R> {
		return new Future(async () => {
			const response = await fetch(
				`/_topcoat/procedures/${encodeURIComponent(this.id)}`,
				{
					method: "POST",
					headers: { "Content-Type": "application/json" },
					body: JSON.stringify(args),
				},
			);
			if (!response.ok) {
				throw new Error(
					`Procedure call failed: ${response.status} ${response.statusText}`,
				);
			}

			return this.cx.s(await response.json()) as R;
		});
	}

	toJSON(): { t: "Procedure"; id: string } {
		return { t: "Procedure", id: this.id };
	}
}
