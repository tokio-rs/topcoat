import { dehydrate } from "./expression/dehydrate";
import type { DehydratedSurrogate } from "./expression/serialized";
import { Effect } from "./reactivity";
import type { RenderUnit } from "./render/unit";
import type { Runtime } from "./runtime";
import type { SignalId } from "./signal-registry";

/**
 * A live region within a scope's content: the range between its marker
 * comments, whose resources the region's own scope owns so a swap can
 * replace them along with the range.
 */
export type Region = {
	id: string;
	start: Comment;
	/** Attached when the end marker is reached. */
	end: Comment | null;
	scope: Scope;
};

/**
 * Owns reactive resources and child scopes. Disposing a scope recursively
 * releases its children and removes any signals it owns from the registry.
 */
export class Scope {
	readonly children = new Set<Scope>();
	/** The ids of the signals declared in this scope's content. */
	readonly signalIds = new Set<SignalId>();
	/**
	 * Signals read on the server while rendering this scope's own content.
	 * A change re-renders the enclosing unit, which watches the
	 * dependencies of its whole content through
	 * [`collectDependencies`](Scope.collectDependencies).
	 */
	readonly dependencies = new Set<SignalId>();
	/**
	 * Whether the server requested a connection while rendering this
	 * scope's own content. The enclosing unit renders again over a
	 * connection.
	 */
	requiresConnection = false;
	/** The live regions directly within this scope's content, by id. */
	readonly regions = new Map<string, Region>();
	private readonly effects = new Set<Effect>();
	/** Aborted on release, removing listeners and cancelling owned requests. */
	private readonly listenerController = new AbortController();
	private disposed = false;

	constructor(
		readonly parent: Scope | null,
		readonly runtime: Runtime,
		/**
		 * The unit whose content this scope belongs to: the one the
		 * dependencies and connection requirements found in it describe.
		 */
		readonly unit: RenderUnit | null = null,
	) {
		parent?.children.add(this);
	}

	/**
	 * The dependencies of this scope's content, including its live regions.
	 * A nested unit watches its own content, so its scopes are left out.
	 */
	collectDependencies(into = new Set<SignalId>()): Set<SignalId> {
		for (const id of this.dependencies) into.add(id);
		for (const child of this.children) {
			if (child.unit === this.unit) child.collectDependencies(into);
		}
		return into;
	}

	/** Whether this scope's content, including its live regions, requires a connection. */
	contentRequiresConnection(): boolean {
		if (this.requiresConnection) return true;
		for (const child of this.children) {
			if (child.unit === this.unit && child.contentRequiresConnection()) {
				return true;
			}
		}
		return false;
	}

	/**
	 * Runs a reaction immediately and owns its subscriptions until release.
	 * Returns the effect, or `null` when the scope is already released.
	 */
	effect(fn: () => void): Effect | null {
		if (this.disposed) return null;
		const effect = new Effect(fn);
		this.effects.add(effect);
		try {
			effect.run();
		} catch (error) {
			effect.dispose();
			this.effects.delete(effect);
			throw error;
		}
		return effect;
	}

	/** Finds the live region `id` in this scope's content or a descendant's. */
	findRegion(id: string): Region | undefined {
		const own = this.regions.get(id);
		if (own !== undefined) return own;
		for (const child of this.children) {
			const found = child.findRegion(id);
			if (found !== undefined) return found;
		}
		return undefined;
	}

	/**
	 * Aborts attached work when this scope is released.
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
		this.regions.clear();

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
