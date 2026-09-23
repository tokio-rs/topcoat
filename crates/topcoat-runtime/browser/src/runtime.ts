import { hydrate as hydrateDom } from "./dom/hydrate";
import { Context } from "./expression/context";
import { PageUnit } from "./render/page";
import type { Scope } from "./scope";
import { type SignalId, SignalRegistry } from "./signal-registry";

export class Runtime {
	readonly registry = new SignalRegistry();
	readonly context: Context = new Context(this.registry);
	/** The page: the outermost unit, owning every signal in the document. */
	readonly page: PageUnit = new PageUnit(this);

	start(root: ParentNode): void {
		this.hydrate(root, null, null, this.page.contentScope);
		this.page.startWatching();
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
