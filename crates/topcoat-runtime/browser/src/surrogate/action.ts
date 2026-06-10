import type { Context } from "../context";

export class Action<A extends unknown[] = unknown[], R = unknown> {
	constructor(
		private readonly cx: Context,
		private readonly id: string,
	) {}

	async call(...args: A): Promise<R> {
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
	}

	toJSON(): { t: "Action"; id: string } {
		return { t: "Action", id: this.id };
	}
}
