import { dehydrate } from "./expression/dehydrate";
import type { DehydratedSurrogate } from "./expression/serialized";
import { Effect } from "./reactivity";
import type { Runtime } from "./runtime";
import type { SignalId } from "./signal-registry";

/**
 * Owns reactive resources and child scopes. Disposing a scope recursively
 * releases its children and removes any signals it owns from the registry.
 */
export class Scope {
	readonly children = new Set<Scope>();
	/** The ids of the signals declared in this scope's content. */
	readonly signalIds = new Set<SignalId>();
	/**
	 * The ids of the signals the content depends on: those the server read
	 * while rendering it, so a change to one re-runs the enclosing unit.
	 */
	readonly dependencies = new Set<SignalId>();
	private readonly effects = new Set<Effect>();
	/** Aborted on release, removing listeners and cancelling owned requests. */
	private readonly listenerController = new AbortController();
	private disposed = false;

	constructor(
		readonly parent: Scope | null,
		readonly runtime: Runtime,
	) {
		parent?.children.add(this);
	}

	/** Runs a reaction immediately and owns its subscriptions until release. */
	effect(fn: () => void): void {
		if (this.disposed) return;
		const effect = new Effect(fn);
		this.effects.add(effect);
		try {
			effect.run();
		} catch (error) {
			effect.dispose();
			this.effects.delete(effect);
			throw error;
		}
	}

	/**
	 * Ends work attached to this scope when it is released: DOM listeners
	 * and, for a render unit's lifetime scope, pending requests.
	 */
	get abortSignal(): AbortSignal {
		return this.listenerController.signal;
	}

	/**
	 * Collects the current values of the signals this scope and its
	 * descendants own, dehydrated for the server, keyed by signal id.
	 */
	collectSignalValues(
		into: Record<SignalId, DehydratedSurrogate> = {},
	): Record<SignalId, DehydratedSurrogate> {
		for (const id of this.signalIds) {
			into[id] = dehydrate(this.runtime.registry.read(id));
		}
		for (const child of this.children) child.collectSignalValues(into);
		return into;
	}

	/**
	 * Disposes this scope and its descendants but keeps the signals they own
	 * registered, returning their ids.
	 *
	 * This is the first half of replacing content: the new content adopts
	 * the signals it declares again, and the caller deletes the rest.
	 */
	release(into: Set<SignalId> = new Set()): Set<SignalId> {
		if (this.disposed) return into;
		this.disposed = true;

		for (const child of this.children) child.release(into);
		this.children.clear();

		for (const effect of this.effects) effect.dispose();
		this.effects.clear();
		this.listenerController.abort();

		for (const id of this.signalIds) into.add(id);
		this.signalIds.clear();

		this.parent?.children.delete(this);
		return into;
	}

	dispose(): void {
		for (const id of this.release()) this.runtime.registry.delete(id);
	}

	get isDisposed(): boolean {
		return this.disposed;
	}
}
