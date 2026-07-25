import {
	createScope,
	effect,
	type Scope as MaverickScope,
	scoped,
	untrack,
} from "@maverick-js/signals";

import type {
	ReactiveScopeId,
	ReactiveScopeTransport,
} from "./comment";
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
 * comments whose content is replaced with shard output whenever any tracked
 * signal changes. HTTP scopes fetch one render per update. WebSocket scopes
 * keep one connection open and can also receive server-pushed renders.
 *
 * The watch effect lives in the reactive scope itself, persisting across
 * re-renders. The content (bindings, declared signals, nested reactive scopes)
 * lives in a child `contentScope` which is disposed and recreated on each
 * replacement.
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
	private socket: WebSocket | null = null;
	private flushPending = false;

	constructor(
		parent: Scope,
		runtime: Runtime,
		readonly scopeId: ReactiveScopeId,
		readonly path: string,
		readonly transport: ReactiveScopeTransport,
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
	 * subscribes to every tracked signal; the first run is represented by the
	 * server-rendered placeholder. A WebSocket sends those initial values once
	 * its connection opens.
	 */
	startWatching(): void {
		if (this.transport === "ws") this.openWebSocket();

		const { context } = this.runtime;
		let first = true;
		this.run(() => {
			effect(() => {
				// Evaluating each parameter inside the effect subscribes to the
				// signals it reads. HTTP's first run is only the initial
				// subscription; WebSocket sends it from the open callback.
				for (const compute of this.computes) compute(context);
				if (first) {
					first = false;
					return;
				}
				if (this.transport === "http") {
					this.scheduleFetch();
				} else if (this.socket?.readyState === WebSocket.OPEN) {
					this.scheduleWebSocketSend();
				}
			});
		});
	}

	override dispose(): void {
		if (this.isDisposed) return;

		this.abortController?.abort();
		this.abortController = null;

		const socket = this.socket;
		this.socket = null;
		if (socket !== null) {
			socket.onopen = null;
			socket.onmessage = null;
			socket.onclose = null;
			socket.onerror = null;
			if (
				socket.readyState === WebSocket.CONNECTING ||
				socket.readyState === WebSocket.OPEN
			) {
				socket.close(1000, "scope disposed");
			}
		}

		super.dispose();
	}

	private dehydrateArguments(): unknown[] {
		const { context } = this.runtime;
		return untrack(() =>
			this.computes.map((compute) =>
				(compute(context) as { dehydrate: () => unknown }).dehydrate(),
			),
		);
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

		let html: string;
		try {
			const res = await fetch(this.path, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify(this.dehydrateArguments()),
				signal: ac.signal,
			});
			html = await res.text();
		} catch (e) {
			if ((e as Error).name === "AbortError") return;
			throw e;
		}

		if (this.isDisposed || this.abortController !== ac) return;
		this.abortController = null;
		this.replaceContent(html);
	}

	private openWebSocket(): void {
		const url = new URL(this.path, window.location.href);
		url.protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
		const socket = new WebSocket(url);
		this.socket = socket;

		socket.onopen = () => {
			if (this.isDisposed || this.socket !== socket) {
				socket.close(1000, "scope disposed");
				return;
			}
			this.sendWebSocketArguments(socket);
		};
		socket.onmessage = (event) => {
			if (
				this.isDisposed ||
				this.socket !== socket ||
				typeof event.data !== "string"
			) {
				return;
			}
			this.replaceContent(event.data);
		};
		socket.onclose = () => {
			if (this.socket === socket) this.socket = null;
		};
	}

	private scheduleWebSocketSend(): void {
		if (this.flushPending) return;
		this.flushPending = true;
		queueMicrotask(() => {
			this.flushPending = false;
			if (this.isDisposed) return;
			const socket = this.socket;
			if (socket?.readyState === WebSocket.OPEN) {
				this.sendWebSocketArguments(socket);
			}
		});
	}

	private sendWebSocketArguments(socket: WebSocket): void {
		socket.send(JSON.stringify(this.dehydrateArguments()));
	}

	private replaceContent(html: string): void {
		if (this.isDisposed || this.endNode === null) return;

		const parent = this.startNode.parentNode;
		const end = this.endNode;
		if (!parent) return;

		this.contentScope.dispose();
		this.contentScope = new Scope(this, this.runtime);

		let node: ChildNode | null = this.startNode.nextSibling;
		while (node && node !== end) {
			const next: ChildNode | null = node.nextSibling;
			parent.removeChild(node);
			node = next;
		}
		const fragment = document.createRange().createContextualFragment(html);
		parent.insertBefore(fragment, end);

		scan(parent, this.startNode, end, this.contentScope);
	}
}
