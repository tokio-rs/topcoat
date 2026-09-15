export class Ref<T> {
	constructor(
		private readonly read: () => T,
		private readonly write?: (v: T) => void,
	) {
		return new Proxy(this, {
			get(reference, property, receiver) {
				if (
					property === "deref" ||
					property === "deref_mut" ||
					property === "dehydrate"
				) {
					return reference[property].bind(reference);
				}
				const pointee = reference.read();
				if (property === "clone" && pointee instanceof Ref) {
					return () => pointee;
				}
				const member = Reflect.get(Object(pointee), property);
				if (property === "clone" && member === undefined) {
					return () => receiver;
				}
				return typeof member === "function" ? member.bind(pointee) : member;
			},
		});
	}

	/** Shared references expose the pointee's methods through the proxy. */
	static shared<T>(read: () => T): Ref<T> & T {
		return new Ref(read) as Ref<T> & T;
	}

	deref(): T {
		return this.read();
	}

	deref_mut() {
		if (!this.write) throw new Error("Cannot mutably dereference a shared Ref");
		// TODO
		this.write(this.read());
	}

	dehydrate(): unknown {
		throw new Error("Ref<T> cannot be dehydrated");
	}
}

/** Cloning a container copies its references and clones its owned values. */
export function cloneValue<T>(value: T): T {
	if (value instanceof Ref) return value;
	const owned = value as { clone?: () => T } | undefined;
	return typeof owned?.clone === "function" ? owned.clone() : value;
}
