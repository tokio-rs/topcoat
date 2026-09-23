/**
 * A Rust reference. Reads go through to the current value, and a proxy
 * forwards the value's methods, so a reference can be used like the value.
 */
export class Ref<T> {
	constructor(
		private readonly read: () => T,
		private readonly write?: (v: T) => void,
	) {
		// biome-ignore lint/correctness/noConstructorReturn: References forward the pointee's methods through this proxy.
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

	/** Returns the value the reference points to. */
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
