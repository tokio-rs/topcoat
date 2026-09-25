import { dehydrate } from "./expression/dehydrate";
import type { DehydratedSurrogate } from "./expression/serialized";
import { Effect } from "./reactivity";
import type { RenderToken } from "./render/frames";
import type { RenderUnit } from "./render/unit";
import type { Runtime } from "./runtime";
import type { SignalId } from "./signal-registry";

/**
 * A live region bounded by two comments. Its scope owns the resources
 * that need to be released when the region's content is replaced.
 */
export type Region = {
	id: string;
	start: Comment;
	/** Set when hydration finds the closing comment. */
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
	 * Signals read by the server for this scope's content. The owning unit
	 * uses collectDependencies() to watch these signals across its scopes.
	 */
	readonly dependencies = new Set<SignalId>();
	/**
	 * Records a connection request found in this scope's content.
	 */
	requiresConnection = false;
	/** Regions directly owned by this scope, indexed by region id. */
	readonly regions = new Map<string, Region>();
	private readonly effects = new Set<Effect>();
	/** Aborted on release, removing listeners and cancelling owned requests. */
	private readonly listenerController = new AbortController();
	private disposed = false;

	constructor(
		readonly parent: Scope | null,
		readonly runtime: Runtime,
		/**
		 * The page or shard that owns this scope's content and handles its
		 * signal dependencies and connection requests.
		 */
		readonly unit: RenderUnit | null = null,
		/**
		 * The render that produced this scope's content. Content scanned
		 * into a child scope belongs to the same render unless the child
		 * says otherwise. `null` for the content the document loaded with.
		 */
		readonly render: RenderToken | null = parent?.render ?? null,
	) {
		parent?.children.add(this);
	}

	/**
	 * Collects signal dependencies from this scope and its live regions.
	 * Skips nested units because they watch their own dependencies.
	 */
	collectDependencies(into = new Set<SignalId>()): Set<SignalId> {
		for (const id of this.dependencies) into.add(id);
		for (const child of this.children) {
			if (child.unit === this.unit) child.collectDependencies(into);
		}
		return into;
	}

	/** Checks this scope and its live regions for a connection request. */
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
	 * Runs an effect and keeps its subscriptions until this scope is released.
	 * Returns `null` if the scope has already been released.
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

	/** Looks up a region in this scope or any of its children. */
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
