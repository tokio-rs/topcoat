export class Ref<T> {
	constructor(
		private readonly read: () => T,
		private readonly write: (v: T) => void,
	) {}

	deref(): T {
		return this.read();
	}

	deref_mut() {
		// TODO
		this.write(this.read());
	}

	dehydrate(): unknown {
		throw new Error("Ref<T> cannot be dehydrated");
	}
}
