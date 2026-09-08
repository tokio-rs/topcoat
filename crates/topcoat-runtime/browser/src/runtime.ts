import { Context } from "./context";
import { scan } from "./scan";
import { PageUnit } from "./scope";
import { SignalRegistry } from "./signal";

export class Runtime {
	readonly registry = new SignalRegistry();
	readonly context: Context = new Context(this.registry);
	/** The page: the outermost unit, owning every signal in the document. */
	readonly page: PageUnit = new PageUnit(this);

	start(root: ParentNode): void {
		scan(root, null, null, this.page.contentScope);
		this.page.startWatching();
	}
}
