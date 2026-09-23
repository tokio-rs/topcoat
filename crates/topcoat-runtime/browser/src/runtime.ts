import { hydrate as hydrateDom } from "./dom/hydrate";
import { Context } from "./expression/context";
import { PageUnit } from "./render/page";
import type { Scope } from "./scope";
import { type SignalId, SignalRegistry } from "./signal-registry";

/**
 * The browser runtime: the signal registry, the context compiled expressions
 * run with, and the page unit that owns the document's content.
 */
export class Runtime {
	readonly registry = new SignalRegistry();
	readonly context: Context = new Context(this.registry);
	/** The page: the outermost unit, owning every signal in the document. */
	readonly page: PageUnit = new PageUnit(this);

	/** Hydrates the markup under `root` and starts watching the page's inputs. */
	start(root: ParentNode): void {
		this.hydrate(root, null, null, this.page.contentScope);
		this.page.startWatching();
	}

	/** Logs an error that has no caller to propagate to. */
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
