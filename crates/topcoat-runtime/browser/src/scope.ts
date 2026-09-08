import {
	createScope,
	effect,
	type Scope as MaverickScope,
	scoped,
	untrack,
} from "@maverick-js/signals";

import type { ShardScopeId } from "./comment";
import type { Context } from "./context";
import type { Runtime } from "./runtime";
import { scan } from "./scan";
import type { SignalId } from "./signal";

type Compute = (cx: Context) => unknown;

/** The route a page re-run is requested from, with the page's path appended. */
export const PAGE_ROUTE_PREFIX = "/_topcoat/runtime/pages";
/** The route a shard re-run is requested from, with the shard's id appended. */
export const SHARD_ROUTE_PREFIX = "/_topcoat/runtime/shards";

/**
 * A region of the DOM that owns disposable reactive resources (effects and
 * possibly child scopes). Disposing a scope recursively disposes its children
 * and removes any signals it owns from the registry.
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

	/**
	 * Collects the current values of the signals this scope and its
	 * descendants own, dehydrated for the server, keyed by signal id.
	 */
	collectSignalValues(
		into: Record<SignalId, unknown> = {},
	): Record<SignalId, unknown> {
		for (const id of this.signalIds) {
			const value = this.runtime.registry.read(id) as
				| { dehydrate?: () => unknown }
				| undefined;
			if (typeof value?.dehydrate === "function") {
				into[id] = value.dehydrate();
			}
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

		this.mScope.dispose();

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

/**
 * A part of the page that re-runs on the server when its inputs change: the
 * page itself, or a shard.
 *
 * A unit's content lives in a child `contentScope` holding its bindings,
 * declared signals, nested units, and the dependencies the server read while
 * rendering it. The watch effect subscribes to the unit's inputs, the
 * dependencies among them, and re-fetches the content when one changes. It
 * lives in the content scope and is rebuilt with every replacement, because
 * the new content decides the new dependencies.
 */
export abstract class Unit extends Scope {
	contentScope: Scope;
	/** Names the unit in error messages. */
	protected abstract readonly label: string;
	private abortController: AbortController | null = null;
	private flushPending = false;

	constructor(parent: Scope | null, runtime: Runtime) {
		super(parent, runtime);
		this.contentScope = new Scope(this, runtime);
	}

	/**
	 * Reads the unit's inputs other than its dependencies, so the effect
	 * calling this subscribes to the signals they read.
	 */
	protected abstract readInputs(): void;

	/** Requests the unit's content from the server with its current inputs. */
	protected abstract request(signal: AbortSignal): Promise<Response>;

	/**
	 * Parses `html` into the nodes replacing the content, or returns `null`
	 * to keep the current content because nothing would change.
	 */
	protected abstract prepare(html: string): DocumentFragment | null;

	/**
	 * Swaps `fragment` into the document in place of the current content and
	 * scans it into `scope`, adopting the signals in `adoptable` it declares.
	 */
	protected abstract insert(
		fragment: DocumentFragment,
		scope: Scope,
		adoptable: Set<SignalId>,
	): void;

	/**
	 * Starts the watch effect over the current content. The effect
	 * subscribes to every input; the first run is the initial subscription
	 * and does not fetch.
	 */
	startWatching(): void {
		const { registry } = this.runtime;
		const scope = this.contentScope;
		let first = true;
		scope.run(() => {
			effect(() => {
				this.readInputs();
				for (const id of scope.dependencies) registry.read(id);
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

	protected async fetchAndReplace(): Promise<void> {
		this.abortController?.abort();
		const ac = new AbortController();
		this.abortController = ac;

		let html: string;
		try {
			const res = await this.request(ac.signal);
			if (res.redirected) {
				// A guard sent the run somewhere else, to a login page say.
				// That is a document for another URL, so navigate there
				// instead of splicing it into this page.
				location.assign(res.url);
				return;
			}
			if (!res.ok) {
				throw new Error(
					`${this.label} request failed: ${res.status} ${res.statusText}`,
				);
			}
			html = await res.text();
		} catch (e) {
			if ((e as Error).name === "AbortError") return;
			throw e;
		}

		if (this.isDisposed || this.abortController !== ac) return;
		this.abortController = null;

		this.replaceContent(html);
	}

	/**
	 * Replaces the content with `html`, keeping the signals the new content
	 * declares again.
	 *
	 * The old content's effects are disposed first, so nothing reacts while
	 * the document changes. Its signals stay registered, so an existing one
	 * wins when the new content declares its id and a value the user changed
	 * while the request was in flight survives. The signals the new content
	 * no longer declares are deleted afterwards.
	 */
	replaceContent(html: string): void {
		const fragment = this.prepare(html);
		if (fragment === null) return;

		const orphans = this.contentScope.release();
		this.contentScope = new Scope(this, this.runtime);
		this.insert(fragment, this.contentScope, orphans);
		for (const id of orphans) this.runtime.registry.delete(id);

		this.startWatching();
	}
}

/**
 * A shard: a region delimited by `<!-- ::topcoat::shard::start/end -->`
 * comments whose content is re-fetched from the shard's route with its
 * computed arguments whenever one of its inputs changes.
 */
export class ShardUnit extends Unit {
	protected readonly label = "Shard";
	endNode: Comment | null = null;
	/**
	 * One compiled function per shard parameter, in declaration order. Each
	 * returns the parameter's current (surrogate) value; reading it inside an
	 * effect subscribes to whatever signals it touches.
	 */
	private readonly computes: Compute[];

	constructor(
		parent: Scope,
		runtime: Runtime,
		readonly scopeId: ShardScopeId,
		readonly shard: string,
		readonly identity: string,
		exprs: string[],
		readonly startNode: Comment,
	) {
		super(parent, runtime);
		this.computes = exprs.map(
			(js) => new Function("cx", `return ${js};`) as Compute,
		);
	}

	/** Must be called before `startWatching`. */
	attachEnd(end: Comment): void {
		this.endNode = end;
	}

	protected readInputs(): void {
		const { context } = this.runtime;
		for (const compute of this.computes) compute(context);
	}

	protected request(signal: AbortSignal): Promise<Response> {
		const { context } = this.runtime;
		// The signals the current content created travel with the request,
		// so the server resumes them instead of starting them over.
		const { args, signals } = untrack(() => ({
			args: this.computes.map((compute) =>
				(compute(context) as { dehydrate: () => unknown }).dehydrate(),
			),
			signals: this.contentScope.collectSignalValues(),
		}));
		return fetch(`${SHARD_ROUTE_PREFIX}/${this.shard}`, {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
				"X-Topcoat-Identity": this.identity,
			},
			body: JSON.stringify({ args, signals }),
			signal,
		});
	}

	protected prepare(html: string): DocumentFragment | null {
		if (this.endNode === null || this.startNode.parentNode === null) {
			return null;
		}
		const fragment = document.createRange().createContextualFragment(html);
		if (markup(fragment.childNodes) === markup(this.contentNodes())) {
			return null;
		}
		return fragment;
	}

	protected insert(
		fragment: DocumentFragment,
		scope: Scope,
		adoptable: Set<SignalId>,
	): void {
		const parent = this.startNode.parentNode;
		const end = this.endNode;
		if (parent === null || end === null) return;

		for (const node of this.contentNodes()) parent.removeChild(node);
		parent.insertBefore(fragment, end);

		scan(parent, this.startNode, end, scope, adoptable);
	}

	/** The nodes between the shard's start and end markers. */
	private contentNodes(): ChildNode[] {
		const nodes: ChildNode[] = [];
		let n: ChildNode | null = this.startNode.nextSibling;
		while (n && n !== this.endNode) {
			nodes.push(n);
			n = n.nextSibling;
		}
		return nodes;
	}
}

/**
 * The page: the outermost unit, whose content is the whole document and
 * whose inputs are its URL and its dependencies. A re-run is requested from
 * the pages route and arrives as a full document, whose body replaces the
 * children of `<body>`; the head is left alone.
 */
export class PageUnit extends Unit {
	protected readonly label = "Page";

	constructor(runtime: Runtime) {
		super(null, runtime);
	}

	protected readInputs(): void {}

	protected request(signal: AbortSignal): Promise<Response> {
		// The page owns every signal in the document, directly or through a
		// shard, so its values are the complete set the re-run resumes from.
		const signals = untrack(() => this.contentScope.collectSignalValues());
		// The root page is served at the bare prefix, since the route below
		// it needs at least one path segment.
		const path = location.pathname === "/" ? "" : location.pathname;
		return fetch(`${PAGE_ROUTE_PREFIX}${path}${location.search}`, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({ signals }),
			signal,
		});
	}

	protected prepare(html: string): DocumentFragment | null {
		const doc = new DOMParser().parseFromString(html, "text/html");
		if (doc.body.innerHTML === document.body.innerHTML) return null;
		const fragment = document.createDocumentFragment();
		fragment.append(...Array.from(doc.body.childNodes));
		return fragment;
	}

	protected insert(
		fragment: DocumentFragment,
		scope: Scope,
		adoptable: Set<SignalId>,
	): void {
		document.body.replaceChildren(fragment);
		scan(document, null, null, scope, adoptable);
	}
}

/** Serializes `nodes` the way the browser would, for comparing content. */
function markup(nodes: Iterable<Node>): string {
	const template = document.createElement("template");
	for (const node of nodes) template.content.append(node.cloneNode(true));
	return template.innerHTML;
}
