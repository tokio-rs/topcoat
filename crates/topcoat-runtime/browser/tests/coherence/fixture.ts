import { Context } from "../../src/context";
import { Future } from "../../src/surrogate/future";
import { Panic } from "../../src/surrogate/panic";

type Awaitable = {
	t: "CoherenceFuture";
	pending_once: boolean;
	completion:
		| { kind: "Return"; value: unknown }
		| { kind: "Panic"; value: string };
};

/** Adds test awaitables while delegating ordinary values to real hydration. */
export class FixtureContext extends Context {
	override hydrate(value: unknown): unknown {
		if (
			value === null ||
			typeof value !== "object" ||
			Reflect.get(value, "t") !== "CoherenceFuture"
		) {
			return super.hydrate(value);
		}
		const fixture = value as Awaitable;
		if (
			typeof fixture.pending_once !== "boolean" ||
			(fixture.completion?.kind !== "Return" &&
				fixture.completion?.kind !== "Panic") ||
			(fixture.completion.kind === "Panic" &&
				typeof fixture.completion.value !== "string")
		) {
			throw new Error("Invalid coherence awaitable");
		}
		return new Future(async () => {
			if (fixture.pending_once) await Promise.resolve();
			if (fixture.completion.kind === "Panic") {
				throw new Panic(fixture.completion.value);
			}
			return this.hydrate(fixture.completion.value);
		});
	}
}
