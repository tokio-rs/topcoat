import type { SignalId, SignalRegistry } from "../signal-registry";
import { Option, Result, WriteSignal } from "../surrogate";
import { hydrate } from "./hydrate";
import type { DehydratedSurrogate } from "./serialized";

/**
 * The `cx` object passed into every compiled expression: the interface
 * generated code uses to hydrate values and access the runtime's signals.
 * Expressions execute as ordinary JavaScript with access to browser globals.
 */
export class Context {
	constructor(private readonly registry: SignalRegistry) {}

	/** Converts a value serialized by the server into its runtime object. */
	hydrate(s: unknown) {
		return hydrate(s as DehydratedSurrogate, this);
	}

	/** Returns the signal with this id. Throws if it does not exist. */
	signal(id: SignalId): WriteSignal<unknown> {
		return new WriteSignal(id, this.registry.handle(id));
	}

	/** Creates `Some(v)`. */
	some<T>(v: T): Option<T> {
		return Option.some(v);
	}

	/** Creates `None`. */
	none<T>(): Option<T> {
		return Option.none<T>();
	}

	/** Creates `Ok(v)`. */
	ok<T, E = never>(v: T): Result<T, E> {
		return Result.from_ok(v);
	}

	/** Creates `Err(v)`. */
	err<T = never, E = unknown>(v: E): Result<T, E> {
		return Result.from_err(v);
	}
}
