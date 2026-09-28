import { morph } from "../../../../topcoat-core/browser/morph";
import { parseChildren } from "../dom/fragment";
import type { Effect } from "../reactivity";
import type { Runtime } from "../runtime";
import { Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import type { ConnectionTarget, RerunRequest } from "./connection";
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
 * When the content requests a connection and no enclosing unit's content
 * does, the unit joins the document's connection, which renders it again
 * once the socket opens. While the socket is open, a unit whose content
 * requests a connection re-runs over it on its own, even inside an
 * enclosing unit that is connected too.
 */
export abstract class RenderUnit implements ConnectionTarget {
	protected readonly lifetime: Scope;
	contentScope: Scope;
	/** Names the unit in error messages. */
	protected abstract readonly label: string;
	private readonly requestController: RenderRequest;
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
		this.lifetime.abortSignal.addEventListener(
			"abort",
			() => runtime.connection.remove(this),
			{ once: true },
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

	/** Describes the request that renders the unit with its current inputs. */
	abstract rerunRequest(): RerunRequest;

	/**
	 * Requests the unit's content from the server with its current inputs,
	 * accepting a response of the `accept` media type.
	 */
	protected request(signal: AbortSignal, accept: string): Promise<Response> {
		const { url, headers, body } = this.rerunRequest();
		return fetch(url, {
			method: "POST",
			cache: "no-store",
			headers: { ...headers, Accept: accept },
			body,
			signal,
		});
	}

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
	 * Checks whether an enclosing unit's content requests a connection, so
	 * that unit's connected render includes this unit's content.
	 */
	private get coveredByAncestor(): boolean {
		for (const unit of this.ancestors()) {
			if (unit.requiresConnection) return true;
		}
		return false;
	}

	/**
	 * Uses the document's connection when this content needs it and no
	 * parent unit already provides it. Leaves the connection otherwise.
	 * Waits for the page load or navigation response to finish, so all HTTP
	 * updates arrive before rendering over the connection starts.
	 */
	private syncConnection(): void {
		if (this.isDisposed) return;
		const { connection } = this.runtime;
		if (!this.requiresConnection || this.coveredByAncestor) {
			connection.leave(this);
			return;
		}
		const loaded = this.runtime.whenLoaded(
			() => this.syncConnection(),
			this.lifetime.abortSignal,
		);
		if (loaded) connection.join(this);
	}

	/** Starts a connected render if the content still needs its own. */
	connectionOpened(): void {
		const { connection } = this.runtime;
		if (this.requiresConnection && !this.coveredByAncestor) {
			connection.run(this);
		} else {
			connection.leave(this);
		}
	}

	/**
	 * Checks whether the current content needs a connection. A unit
	 * hydrated inside other content defers the check until the enclosing
	 * content has been scanned, so it can see whether an enclosing unit
	 * connects instead.
	 */
	protected scheduleConnection(): void {
		this.syncConnection();
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
	 * Content that needs a connection re-runs over the document's connection
	 * while it is open, without re-running any enclosing unit. Otherwise the
	 * unit stops its connected run, if any, and posts an HTTP request, whose
	 * response arrives as frames: a snapshot replacing the content, then a
	 * swap for each later update of a live region.
	 */
	refresh(): Promise<void> {
		const { connection } = this.runtime;
		if (this.requiresConnection && connection.run(this)) {
			return Promise.resolve();
		}
		connection.stop(this);
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
