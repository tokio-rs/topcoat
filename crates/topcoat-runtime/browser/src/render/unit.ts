import type { Runtime } from "../runtime";
import { Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import { RenderRequest } from "./request";

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
	 * Replaces the content with `html`, keeping the signals the new content
	 * declares again and every element the new content can be morphed into.
	 *
	 * The old content's effects and listeners are disposed first, so nothing
	 * reacts while the document changes. The new markup is then morphed into
	 * the existing nodes rather than swapped in, so focus, scroll position,
	 * and what the user is typing survive, and the result is scanned again
	 * as if it were fresh content. The old signals stay registered
	 * throughout, so an existing one wins when the new content declares its
	 * id and a value the user changed while the request was in flight
	 * survives. The signals the new content no longer declares are deleted
	 * afterwards.
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
