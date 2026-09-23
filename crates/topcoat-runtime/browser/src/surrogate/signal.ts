import { dehydrate } from "../expression/dehydrate";
import type { DehydratedSurrogate } from "../expression/serialized";
import type { WriteSignal as SignalHandle } from "../reactivity";
import type { SignalId } from "../signal-registry";
import type { Bool } from "./bool";
import { F64 } from "./f64";
import { Integer } from "./integer";
import { cloneValue, Ref } from "./ref";
import { String as RuntimeString, type Str } from "./string";

/**
 * A Rust `Signal` inside a compiled expression: reads and writes of one
 * signal in the registry.
 */
export class WriteSignal<T> {
	constructor(
		private readonly id: SignalId,
		private readonly inner: SignalHandle<T>,
	) {}

	/** Borrows the current value, tracking the read. */
	read(): Ref<T> {
		return new Ref(
			() => this.inner(),
			(v) => this.inner.set(v),
		);
	}

	/** Clones the current value, tracking the read. */
	get(): T {
		return cloneValue(this.read().deref());
	}

	/** Replaces the value. */
	set(v: T): void {
		this.inner.set(v);
	}

	/** Negates a `bool` value. */
	toggle(): void {
		this.inner.set((prev) => (prev as Bool).not() as T);
	}

	/** Adds one to a numeric value. */
	increment(): void {
		this.inner.set(
			(prev) =>
				(prev instanceof Integer
					? prev.increment()
					: (prev as F64).add(new F64(1))) as T,
		);
	}

	/** Subtracts one from a numeric value. */
	decrement(): void {
		this.inner.set(
			(prev) =>
				(prev instanceof Integer
					? prev.decrement()
					: (prev as F64).sub(new F64(1))) as T,
		);
	}

	/** Appends a string to a `String` value. */
	push_str(s: Str): void {
		this.inner.set((prev) => new RuntimeString(`${prev}${s}`) as T);
	}

	/**
	 * The form the signal takes as an argument to a run on the server: its
	 * id next to its current value, so the server can rebuild the signal
	 * without holding the value itself.
	 */
	dehydrate(): { t: "Signal"; id: SignalId; v: DehydratedSurrogate } {
		return { t: "Signal", id: this.id, v: dehydrate(this.inner()) };
	}
}
