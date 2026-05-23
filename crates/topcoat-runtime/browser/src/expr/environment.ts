export class Environment {
	private bindings: Map<string, unknown>;

	constructor(private parent: Environment | undefined) {
		this.bindings = new Map<string, unknown>();
	}

	public define(name: string, value: unknown) {
		this.bindings.set(name, value);
	}

	public get(name: string) {
		if (this.bindings.has(name)) {
			return this.bindings.get(name);
		}

		if (this.parent !== undefined) {
			return this.parent.get(name);
		}

		throw new Error(`Variable "${name}" is not defined.`);
	}

	public assign(name: string, value: unknown) {
		if (this.bindings.has(name)) {
			this.bindings.set(name, value);
			return;
		}

		if (this.parent !== undefined) {
			this.parent.assign(name, value);
			return;
		}

		throw new ReferenceError(`Cannot assign to undefined variable "${name}".`);
	}

	public getParent() {
		return this.parent;
	}
}
