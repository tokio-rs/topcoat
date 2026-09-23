import { dehydrate } from "../expression/dehydrate";
import type { DehydratedSurrogate } from "../expression/serialized";
import type { WriteSignal as SignalHandle } from "../reactivity";
import type { SignalId } from "../signal-registry";
import type { Bool } from "./bool";
import { F64 } from "./f64";
import { Integer } from "./integer";
import { cloneValue, Ref } from "./ref";
import { String as RuntimeString, type Str } from "./string";

export class WriteSignal<T> {
	constructor(
		private readonly id: SignalId,
		private readonly inner: SignalHandle<T>,
	) {}

	read(): Ref<T> {
		return new Ref(
			() => this.inner(),
			(v) => this.inner.set(v),
		);
	}

	get(): T {
		return cloneValue(this.read().deref());
	}

	set(v: T): void {
		this.inner.set(v);
	}

	toggle(): void {
		this.inner.set((prev) => (prev as Bool).not() as T);
	}

	increment(): void {
		this.inner.set(
			(prev) =>
				(prev instanceof Integer
					? prev.increment()
					: (prev as F64).add(new F64(1))) as T,
		);
	}

	decrement(): void {
		this.inner.set(
			(prev) =>
				(prev instanceof Integer
					? prev.decrement()
					: (prev as F64).sub(new F64(1))) as T,
		);
	}

	push_str(s: Str): void {
		this.inner.set((prev) => new RuntimeString(`${prev}${s}`) as T);
	}

	/**
	 * Sends the signal's id and current value so the server can restore it.
	 */
	dehydrate(): { t: "Signal"; id: SignalId; v: DehydratedSurrogate } {
		return { t: "Signal", id: this.id, v: dehydrate(this.inner()) };
	}
}
