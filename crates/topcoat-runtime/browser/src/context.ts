import type { Scope } from "./scope";
import type { SignalId, SignalRegistry } from "./signal";
import {
	type DehydratedSurrogate,
	hydrateSurrogate,
	Option,
	Result,
	WriteSignal,
} from "./surrogate";
import { type SwapOptions, swapHtml } from "./swap";

/**
 * The `cx` object passed into every compiled expression. It is the only
 * way generated code can reach back into the runtime, keeping the surface
 * narrow makes the generated JS easy to audit and keeps non-context globals
 * inaccessible from inside `new Function`.
 */
export class Context {
	/**
	 * The scope swapped fragments attach to. Set by the runtime once its root
	 * scope exists; `null` for throwaway contexts that never swap, such as the
	 * one used to hydrate a signal marker value.
	 */
	swapScope: Scope | null = null;

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

	/**
	 * Replaces a DOM region with an HTML fragment and re-attaches Topcoat
	 * bindings and event handlers.
	 *
	 * See {@link swapHtml} for details.
	 */
	swapHtml(selector: string, html: string, opts?: SwapOptions): void {
		if (!this.swapScope) {
			throw new Error("swapHtml: no swap scope is attached to this context");
		}
		swapHtml(this.swapScope, selector, html, opts);
	}
}
