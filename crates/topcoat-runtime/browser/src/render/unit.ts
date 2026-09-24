import { morph } from "../../../../topcoat-core/browser/morph";
import { parseChildren } from "../dom/fragment";
import type { Effect } from "../reactivity";
import type { Runtime } from "../runtime";
import { Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import { Connection, type ConnectionTarget } from "./connection";
import {
	applyFrame,
	FRAMES_MEDIA_TYPE,
	newRender,
	type RenderToken,
} from "./frames";
import { RenderRequest } from "./request";

/**
 * Content that re-renders on the server when its inputs change.
 *
 * `contentScope` owns the current content's resources. Its watch effect
 * subscribes to inputs and server-read dependencies. Replacing content rebuilds
 * the scope and effect so subscriptions reflect the new content.
 *
 * When the content requests a connection and no enclosing unit provides
 * one, the unit opens a WebSocket at its URL and uses it for renders while
 * connected. It keeps the connection for the rest of its lifetime, even if
 * later content no longer needs it.
 */
export abstract class RenderUnit implements ConnectionTarget {
	protected readonly lifetime: Scope;
	contentScope: Scope;
	/** Names the unit in error messages. */
	protected abstract readonly label: string;
	private readonly requestController: RenderRequest;
	private connection: Connection | null = null;
	/** Watches the signals used by the current content. */
	private watch: Effect | null = null;
	/**
	 * Prevents a render request while the effect updates its subscriptions.
	 */
	private subscribing = false;

	constructor(
		parent: Scope | null,
		readonly runtime: Runtime,
	) {
		this.lifetime = new Scope(parent, runtime, this);
		this.contentScope = new Scope(this.lifetime, runtime, this);
		this.requestController = new RenderRequest(
			this.lifetime.abortSignal,
			(error) => runtime.reportError(error),
		);
	}

	get isDisposed(): boolean {
		return this.lifetime.isDisposed;
	}

	/**
	 * Checks whether the current content contains a connection marker.
	 * Replacing the content can change the result.
	 */
	get requiresConnection(): boolean {
		return this.contentScope.contentRequiresConnection();
	}

	dispose(): void {
		this.lifetime.dispose();
	}

	/**
	 * Reads the unit's inputs other than its dependencies, so the effect
	 * calling this subscribes to the signals they read.
	 */
	protected abstract readInputs(): void;

	/**
	 * Requests the unit's content from the server with its current inputs,
	 * accepting a response of the `accept` media type.
	 */
	protected abstract request(
		signal: AbortSignal,
		accept: string,
	): Promise<Response>;

	/**
	 * Parses `html` into the nodes the content becomes, or returns `null` to
	 * keep the current content because the unit is no longer in the document.
	 */
	protected abstract prepare(html: string): Node[] | null;

	/**
	 * Morphs the current content into `nodes` and scans the result into
	 * `scope`, adopting the signals in `adoptable` it declares.
	 */
	protected abstract insert(
		nodes: Node[],
		scope: Scope,
		adoptable: Set<SignalId>,
	): void;

	/** Returns the HTTP URL of the unit's renders, where it connects. */
	protected abstract url(): string;

	/** Collects the fields to send with a render request. */
	abstract renderInputs(): object;

	reportError(error: unknown): void {
		this.runtime.reportError(error);
	}

	/** Returns the units enclosing this one, innermost first. */
	private *ancestors(): Generator<RenderUnit> {
		for (let scope = this.lifetime.parent; scope; scope = scope.parent) {
			if (scope.unit !== null && scope.unit !== this) yield scope.unit;
		}
	}

	/**
	 * Checks whether an enclosing unit renders this unit's content over its
	 * own connection, or is about to.
	 */
	private get coveredByAncestor(): boolean {
		for (const unit of this.ancestors()) {
			if (unit.connection !== null || unit.requiresConnection) return true;
		}
		return false;
	}

	/**
	 * Opens a connection if the content needs one and no enclosing unit
	 * provides it. Waits for the document to finish loading so all initial
	 * HTTP updates arrive before the first render over the connection.
	 */
	private connectIfRequired(loaded = document.readyState === "complete"): void {
		if (this.isDisposed || this.connection !== null) return;
		if (!this.requiresConnection) return;
		if (!loaded) {
			window.addEventListener("load", () => this.connectIfRequired(true), {
				once: true,
				signal: this.lifetime.abortSignal,
			});
			return;
		}
		if (this.coveredByAncestor) return;
		const url = new URL(this.url(), location.href);
		url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
		this.connection = new Connection(
			url.href,
			this,
			this.lifetime.abortSignal,
		);
	}

	/**
	 * Checks whether the current content needs a connection. A unit
	 * hydrated inside other content defers the check until the enclosing
	 * content has been scanned, so it can see whether an enclosing unit
	 * connects instead.
	 */
	protected scheduleConnection(): void {
		this.connectIfRequired();
	}

	/**
	 * Starts the watch effect over the current content. The effect
	 * subscribes to every input; the first run is the initial subscription
	 * and does not fetch.
	 */
	startWatching(): void {
		const { registry } = this.runtime;
		const scope = this.contentScope;
		this.subscribing = true;
		try {
			this.watch = scope.effect(() => {
				this.readInputs();
				for (const id of scope.collectDependencies()) registry.read(id);
				if (this.subscribing) return;
				this.requestController.schedule(() => this.refresh());
			});
		} finally {
			this.subscribing = false;
		}
		this.scheduleConnection();
	}

	/**
	 * Updates the effect's subscriptions and connection after a swap
	 * changes what the content depends on.
	 */
	resubscribe(): void {
		this.subscribing = true;
		try {
			this.watch?.run();
		} finally {
			this.subscribing = false;
		}
		this.scheduleConnection();
	}

	/**
	 * Re-runs this unit immediately with its current inputs.
	 *
	 * While the unit's connection is open, the run goes over it. Content
	 * that needs a connection inside a connected unit re-runs that unit, so
	 * it stays connected. Otherwise the unit posts an HTTP request, whose
	 * response arrives as frames: a snapshot replacing the content, then a
	 * swap for each later update of a live region.
	 */
	refresh(): Promise<void> {
		if (this.connection?.isOpen) {
			this.connection.requestRun();
			return Promise.resolve();
		}
		if (this.requiresConnection) {
			for (const unit of this.ancestors()) {
				if (unit.connection?.isOpen) return unit.refresh();
			}
		}
		const render = newRender();
		return this.requestController.run(
			(signal) => this.request(signal, FRAMES_MEDIA_TYPE),
			(frame) => applyFrame(this, frame, this.label, render),
			this.label,
		);
	}

	/**
	 * Replaces content with `html`, preserving matching elements and signals.
	 *
	 * Disposes the old effects and listeners before updating the DOM, then
	 * hydrates the result. Existing signal values take precedence over new
	 * declarations, preserving changes made while the request was pending.
	 * Signals absent from the new content are deleted. The new content
	 * belongs to `render`.
	 */
	replaceContent(html: string, render: RenderToken): void {
		if (this.isDisposed) return;
		const nodes = this.prepare(html);
		if (nodes === null) return;
		this.replace(
			(scope, orphans) => this.insert(nodes, scope, orphans),
			render,
		);
	}

	/**
	 * Replaces a live region's content and hydrates the new HTML.
	 * Releases the old content's resources and preserves signals declared
	 * again in the replacement.
	 *
	 * Ignores updates for regions that are no longer in the current content,
	 * and for regions a different render produced: a nested unit that has
	 * re-rendered on its own owns its regions until this unit renders again.
	 */
	applySwap(id: string, html: string, render: RenderToken | null): void {
		if (this.isDisposed) return;
		const region = this.contentScope.findRegion(id);
		if (region === undefined || region.end === null) return;
		if (region.scope.render !== render) return;
		const parent = region.start.parentNode;
		if (parent === null) return;
		const nodes = parseChildren(parent, html);

		const orphans = region.scope.release();
		region.scope = new Scope(
			region.scope.parent,
			this.runtime,
			region.scope.unit,
		);
		morph(parent, region.start, region.end, nodes);
		this.runtime.hydrate(
			parent,
			region.start,
			region.end,
			region.scope,
			orphans,
		);
		for (const id of orphans) this.runtime.registry.delete(id);

		// Update the owning unit's subscriptions, including for nested shards.
		region.scope.unit?.resubscribe();
	}

	/**
	 * Rebuilds the content's resources around a DOM update, making the
	 * result the content of `render`.
	 */
	protected replace(
		insert: (scope: Scope, orphans: Set<SignalId>) => void,
		render: RenderToken,
	): void {
		if (this.isDisposed) return;
		this.requestController.cancel();

		const orphans = this.contentScope.release();
		this.contentScope = new Scope(this.lifetime, this.runtime, this, render);
		insert(this.contentScope, orphans);
		for (const id of orphans) this.runtime.registry.delete(id);

		this.startWatching();
	}
}
