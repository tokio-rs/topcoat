import type { SignalId, SignalRegistry } from "./signal";
import {
	type DehydratedSurrogate,
	hydrateSurrogate,
	Option,
	Result,
	WriteSignal,
} from "./surrogate";

/**
 * The `cx` object passed into every compiled expression. It is the only
 * way generated code can reach back into the runtime, keeping the surface
 * narrow makes the generated JS easy to audit and keeps non-context globals
 * inaccessible from inside `new Function`.
 */
export class Context {
	constructor(private readonly registry: SignalRegistry) {}

	getRegistry(): SignalRegistry {
		return this.registry;
	}

	hydrate(s: unknown) {
		return hydrateSurrogate(s as DehydratedSurrogate, this);
	}

	signal(id: SignalId): WriteSignal<unknown> {
		return new WriteSignal(id, this.registry.handle(id));
	}

	some<T>(v: T): Option<T> {
		return Option.some(v);
	}

	none<T>(): Option<T> {
		return Option.none<T>();
	}

	ok<T, E = never>(v: T): Result<T, E> {
		return Result.from_ok(v);
	}

	err<T = never, E = unknown>(v: E): Result<T, E> {
		return Result.from_err(v);
	}
}
