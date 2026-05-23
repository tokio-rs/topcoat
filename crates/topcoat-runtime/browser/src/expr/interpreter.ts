import type { WriteSignal } from "@maverick-js/signals";
import type { SignalId, SignalRegistry } from "../signal";
import { Environment } from "./environment";

export class Interpreter {
	private environment: Environment;

	public constructor(private readonly registry: SignalRegistry) {
		this.environment = new Environment(undefined);
	}

	public pushEnvironment() {
		this.environment = new Environment(this.environment);
	}

	public popEnvironment() {
		const parent = this.environment.getParent();
		if (parent === undefined)
			throw new Error("Tried to pop outermost environment");
		this.environment = parent;
	}

	public getEnvironment() {
		return this.environment;
	}

	public getSignal(id: SignalId): WriteSignal<unknown> {
		return this.registry.handle(id);
	}
}
