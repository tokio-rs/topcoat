import type { SignalId, SignalRegistry } from "./signal";
import {
	deserializeSurrogate,
	// biome-ignore lint/suspicious/noShadowRestrictedNames: Surrogate type
	Option,
	type SerializedSurrogate,
	WriteSignal,
} from "./surrogate";

/**
 * The `cx` object passed into every compiled expression. It is the only
 * way generated code can reach back into the runtime — keeping the surface
 * narrow makes the generated JS easy to audit and keeps non-context globals
 * inaccessible from inside `new Function`.
 */
export class Context {
	constructor(private readonly registry: SignalRegistry) {}

	s(s: unknown) {
		return deserializeSurrogate(s as SerializedSurrogate, this.registry);
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
}
