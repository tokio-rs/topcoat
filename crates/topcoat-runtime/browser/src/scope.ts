import {
	createScope,
	effect,
	type Scope as MaverickScope,
	scoped,
} from "@maverick-js/signals";

import type { ReactiveScopeId } from "./comment";

export type HandleId = string;

export class Scope {
	private readonly children = new Set<Scope>();
	private readonly handles = new Map<HandleId, unknown>();
	private readonly mScope: MaverickScope = createScope();
	private disposed = false;

	constructor(readonly parent: Scope | null) {
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

		this.parent?.children.delete(this);
	}

	get isDisposed(): boolean {
		return this.disposed;
	}

	pushHandle(id: HandleId, value: unknown) {
		this.handles.set(id, value);
	}

	handle(id: HandleId): unknown | undefined {
		const handle = this.handles.get(id);
		if (handle) return handle;
		if (this.parent) return this.parent.handle(id);
		return undefined;
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
	private abortController: AbortController | null = null;
	private flushPending = false;

	constructor(
		parent: Scope,
		readonly scopeId: ReactiveScopeId,
		readonly track: HandleId[],
		readonly path: string,
		readonly startNode: Comment,
	) {
		super(parent);
		this.contentScope = new Scope(this);
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
		let first = true;
		this.run(() => {
			effect(() => {
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

		// const params = untrack(() =>
		// 	this.track.map((id) => ({
		// 		id,
		// 		value: (
		// 			this.runtime.registry.get(id)?.() as { dehydrate: () => unknown }
		// 		).dehydrate(),
		// 	})),
		// );
		// const url = `${this.path}?signals=${encodeURIComponent(JSON.stringify(params))}`;
		//
		// let html: string;
		// try {
		// 	const res = await fetch(url, { signal: ac.signal });
		// 	html = await res.text();
		// } catch (e) {
		// 	if ((e as Error).name === "AbortError") return;
		// 	throw e;
		// }
		//
		// if (this.isDisposed || this.abortController !== ac) return;
		// this.abortController = null;
		//
		// const parent = this.startNode.parentNode;
		// const end = this.endNode;
		// if (!parent) return;
		//
		// this.contentScope.dispose();
		// this.contentScope = new Scope(this);
		//
		// let n: ChildNode | null = this.startNode.nextSibling;
		// while (n && n !== end) {
		// 	const next: ChildNode | null = n.nextSibling;
		// 	parent.removeChild(n);
		// 	n = next;
		// }
		// const fragment = document.createRange().createContextualFragment(html);
		// parent.insertBefore(fragment, end);
		//
		// scan(parent, this.startNode, end, this.contentScope);
	}
}
