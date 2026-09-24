import { morph } from "../../../../topcoat-core/browser/morph";
import type { Effect } from "../reactivity";
import type { Runtime } from "../runtime";
import { Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import { RenderRequest } from "./request";

/**
 * Content that re-renders on the server when its inputs change.
 *
 * `contentScope` owns the current content's resources. Its watch effect
 * subscribes to inputs and server-read dependencies. Replacing content rebuilds
 * the scope and effect so subscriptions reflect the new content.
 */
export abstract class RenderUnit {
	protected readonly lifetime: Scope;
	contentScope: Scope;
	/** Names the unit in error messages. */
	protected abstract readonly label: string;
	private readonly requestController: RenderRequest;
	/** The effect watching the current content's inputs. */
	private watch: Effect | null = null;
	/**
	 * Set while the watch effect runs only to subscribe, so it does not
	 * request a re-render.
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
	 * Whether the current content asked for a server connection. Replacing
	 * the content re-evaluates this from the new content's markers.
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

	/** Requests the unit's content from the server with its current inputs. */
	protected abstract request(signal: AbortSignal): Promise<Response>;

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
	}

	/**
	 * Subscribes the watch effect to the inputs again, after a swap changed
	 * the dependencies of the current content.
	 */
	resubscribe(): void {
		this.subscribing = true;
		try {
			this.watch?.run();
		} finally {
			this.subscribing = false;
		}
	}

	/** Re-runs this unit immediately with its current inputs. */
	refresh(): Promise<void> {
		return this.requestController.run(
			(signal) => this.request(signal),
			(html) => this.replaceContent(html),
			this.label,
		);
	}

	/**
	 * Replaces content with `html`, preserving matching elements and signals.
	 *
	 * Disposes the old effects and listeners before updating the DOM, then
	 * hydrates the result. Existing signal values take precedence over new
	 * declarations, preserving changes made while the request was pending.
	 * Signals absent from the new content are deleted.
	 */
	replaceContent(html: string): void {
		if (this.isDisposed) return;
		const nodes = this.prepare(html);
		if (nodes === null) return;
		this.replace((scope, orphans) => this.insert(nodes, scope, orphans));
	}

	/**
	 * Replaces the content of the live region `id` with `html`, rebuilding
	 * the region's resources around the update the way a whole replacement
	 * does for the unit.
	 *
	 * A region the current content does not have is ignored: the content
	 * changed shape since the swap was produced, and a snapshot follows.
	 */
	applySwap(id: string, html: string): void {
		if (this.isDisposed) return;
		const region = this.contentScope.findRegion(id);
		if (region === undefined || region.end === null) return;
		const parent = region.start.parentNode;
		if (parent === null) return;
		const fragment = document.createRange().createContextualFragment(html);
		const nodes = Array.from(fragment.childNodes);

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

		// The swap changed the dependencies of the unit the region belongs
		// to, which is a nested unit when the region lies inside one.
		region.scope.unit?.resubscribe();
	}

	/** Rebuilds the content's resources around a DOM update. */
	protected replace(
		insert: (scope: Scope, orphans: Set<SignalId>) => void,
	): void {
		if (this.isDisposed) return;
		this.requestController.cancel();

		const orphans = this.contentScope.release();
		this.contentScope = new Scope(this.lifetime, this.runtime, this);
		insert(this.contentScope, orphans);
		for (const id of orphans) this.runtime.registry.delete(id);

		this.startWatching();
	}
}
