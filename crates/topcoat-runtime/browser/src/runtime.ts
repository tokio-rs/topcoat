import { Context } from "./context";
import { scan } from "./scan";
import { Scope } from "./scope";
import { SignalRegistry } from "./signal";

export class Runtime {
	readonly registry = new SignalRegistry();
	readonly context: Context = new Context(this.registry);
	readonly rootScope: Scope = new Scope(null, this);

	start(root: ParentNode): void {
		scan(root, null, null, this.rootScope);
	}
}
