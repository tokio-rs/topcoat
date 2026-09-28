import { hydrate as hydrateDom } from "./dom/hydrate";
import { Context } from "./expression/context";
import { Connection } from "./render/connection";
import { NavigationController } from "./render/navigation";
import { PageUnit, pageUrl } from "./render/page";
import type { Scope } from "./scope";
import { type SignalId, SignalRegistry } from "./signal-registry";

export class Runtime {
	readonly registry = new SignalRegistry();
	readonly context: Context = new Context(this.registry);
	/**
	 * The document's WebSocket, opened at the page's URL, carrying the
	 * connected renders of the page and its shards.
	 */
	readonly connection: Connection = new Connection(pageUrl, (error) =>
		this.reportError(error),
	);
	/** The page: the outermost unit, owning every signal in the document. */
	readonly page: PageUnit = new PageUnit(this);
	/** Navigates between pages without loading a new document. */
	readonly navigation: NavigationController = new NavigationController(this);
	/** Whether the document's `load` event has fired. */
	private loaded = false;

	start(root: ParentNode): void {
		this.hydrate(root, null, null, this.page.contentScope);
		this.page.startWatching();
		this.navigation.start();
	}

	/**
	 * Checks whether the document has finished loading, including the
	 * response of a navigation in progress. If it has not, calls `callback`
	 * once it has, unless `signal` aborts first.
	 */
	whenLoaded(callback: () => void, signal: AbortSignal): boolean {
		if (!this.loaded && document.readyState !== "complete") {
			const loaded = () => {
				this.loaded = true;
				callback();
			};
			window.addEventListener("load", loaded, { once: true, signal });
			return false;
		}
		const pending = this.navigation.pending;
		if (pending === null) return true;
		void pending.then(() => {
			if (!signal.aborted) callback();
		});
		return false;
	}

	reportError(error: unknown): void {
		console.error("[topcoat]", error);
	}

	/** Attaches the markup's resources to its owning scope. */
	hydrate(
		root: Node,
		from: Node | null,
		to: Node | null,
		scope: Scope,
		adoptable: Set<SignalId> = new Set(),
	): void {
		hydrateDom(root, from, to, scope, adoptable);
	}
}
