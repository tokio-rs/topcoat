import {
	createScope,
	effect,
	type Scope as MaverickScope,
	scoped,
	untrack,
} from "@maverick-js/signals";

import type { ReactiveScopeId } from "./comment";
import type { Context } from "./context";
import type { Runtime } from "./runtime";
import { scan } from "./scan";
import type { SignalId } from "./signal";

type Compute = (cx: Context) => unknown;

/**
 * A region of the DOM that owns disposable reactive resources (effects and
 * possibly child scopes). Disposing a scope recursively disposes its children
 * and removes any signals it owns from the registry.
 */
export class Scope {
	readonly children = new Set<Scope>();
	readonly signalIds = new Set<SignalId>();
	private readonly mScope: MaverickScope = createScope();
	private disposed = false;

	constructor(
		readonly parent: Scope | null,
		readonly runtime: Runtime,
	) {
		parent?.children.add(this);
	}

	/** Runs `fn` inside this scope so effects it creates attach for disposal. */
	run<T>(fn: () => T): T {
		return scoped(fn, this.mScope) as T;
	}

	dispose(): void {
		if (this.disposed) return;
		this.disposed = true;

		for (const child of this.children) child.dispose();
		this.children.clear();

		this.mScope.dispose();

		for (const id of this.signalIds) this.runtime.registry.delete(id);
		this.signalIds.clear();

		this.parent?.children.delete(this);
	}

	get isDisposed(): boolean {
		return this.disposed;
	}
}

/**
 * A reactive scope: a region delimited by `<!-- ::topcoat::scope::start/end -->`
 * comments whose content is re-fetched from the server whenever any tracked
 * signal changes.
 *
 * The watch effect lives in the reactive scope itself, persisting across
 * re-renders. The content (bindings, declared signals, nested reactive scopes)
 * lives in a child `contentScope` which is disposed and recreated on each
 * fetch.
 */
export class ReactiveScope extends Scope {
	contentScope: Scope;
	endNode: Comment | null = null;
	/**
	 * One compiled function per shard parameter, in declaration order. Each
	 * returns the parameter's current (surrogate) value; reading it inside an
	 * effect subscribes to whatever signals it touches.
	 */
	private readonly computes: Compute[];
	private abortController: AbortController | null = null;
	private flushPending = false;

	constructor(
		parent: Scope,
		runtime: Runtime,
		readonly scopeId: ReactiveScopeId,
		readonly path: string,
		exprs: string[],
		readonly startNode: Comment,
	) {
		super(parent, runtime);
		this.contentScope = new Scope(this, runtime);
		this.computes = exprs.map(
			(js) => new Function("cx", `return ${js};`) as Compute,
		);
	}

	attachEnd(end: Comment): void {
		this.endNode = end;
	}

	/**
	 * Starts the watch effect. Must be called after `attachEnd`. The effect
	 * subscribes to every tracked signal; the first run is the initial
	 * subscription and does not fetch.
	 */
	startWatching(): void {
		const { context } = this.runtime;
		let first = true;
		this.run(() => {
			effect(() => {
				// Evaluating each parameter inside the effect subscribes to the
				// signals it reads, so the scope re-fetches when any of them
				// change. The first run is just the initial subscription.
				for (const compute of this.computes) compute(context);
				if (first) {
					first = false;
					return;
				}
				this.scheduleFetch();
			});
		});
	}

	private scheduleFetch(): void {
		if (this.flushPending) return;
		this.flushPending = true;
		queueMicrotask(() => {
			this.flushPending = false;
			if (this.isDisposed) return;
			void this.fetchAndReplace();
		});
	}

	private async fetchAndReplace(): Promise<void> {
		if (this.endNode === null) return;

		this.abortController?.abort();
		const ac = new AbortController();
		this.abortController = ac;

		const { context } = this.runtime;
		const args = untrack(() =>
			this.computes.map((compute) =>
				(compute(context) as { dehydrate: () => unknown }).dehydrate(),
			),
		);

		let html: string;
		try {
			const res = await fetch(this.path, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify(args),
				signal: ac.signal,
			});
			html = await res.text();
		} catch (e) {
			if ((e as Error).name === "AbortError") return;
			throw e;
		}

		if (this.isDisposed || this.abortController !== ac) return;
		this.abortController = null;

		const parent = this.startNode.parentNode;
		const end = this.endNode;
		if (!parent) return;

		this.contentScope.dispose();
		this.contentScope = new Scope(this, this.runtime);

		let n: ChildNode | null = this.startNode.nextSibling;
		while (n && n !== end) {
			const next: ChildNode | null = n.nextSibling;
			parent.removeChild(n);
			n = next;
		}
		const fragment = document.createRange().createContextualFragment(html);
		parent.insertBefore(fragment, end);

		scan(parent, this.startNode, end, this.contentScope);
	}
}
