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

	constructor(
		parent: Scope | null,
		readonly runtime: Runtime,
	) {
		this.lifetime = new Scope(parent, runtime);
		this.contentScope = new Scope(this.lifetime, runtime);
		this.requestController = new RenderRequest(
			this.lifetime.abortSignal,
			(error) => runtime.reportError(error),
		);
	}

	get isDisposed(): boolean {
		return this.lifetime.isDisposed;
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
		let first = true;
		scope.effect(() => {
			this.readInputs();
			for (const id of scope.dependencies) registry.read(id);
			if (first) {
				first = false;
				return;
			}
			this.requestController.schedule(() => this.refresh());
		});
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

	/** Rebuilds the content's resources around a DOM update. */
	protected replace(
		insert: (scope: Scope, orphans: Set<SignalId>) => void,
	): void {
		if (this.isDisposed) return;
		this.requestController.cancel();

		const orphans = this.contentScope.release();
		this.contentScope = new Scope(this.lifetime, this.runtime);
		insert(this.contentScope, orphans);
		for (const id of orphans) this.runtime.registry.delete(id);

		this.startWatching();
	}
}
