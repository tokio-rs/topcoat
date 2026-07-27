import type { WriteSignal as MaverickWriteSignal } from "@maverick-js/signals";

import type { SignalId } from "../signal";
import { Bool } from "./bool";
import { F64 } from "./f64";
import { Ref } from "./ref";
import { String, type Str } from "./string";

export class WriteSignal<T> {
	constructor(
		private readonly id: SignalId,
		private readonly inner: MaverickWriteSignal<T>,
	) {}

	read(): Ref<T> {
		return new Ref(
			() => this.inner(),
			(v) => this.inner.set(v),
		);
	}

	get(): T {
		const value = this.read().deref() as { clone?: () => T };
		return typeof value?.clone === "function" ? value.clone() : (value as T);
	}

	set(v: T): void {
		this.inner.set(v);
	}

	toggle(): void {
		this.inner.set((prev) => (prev as Bool).not() as T);
	}

	increment(): void {
		this.inner.set((prev) => (prev as F64).add(new F64(1)) as T);
	}

	decrement(): void {
		this.inner.set((prev) => (prev as F64).sub(new F64(1)) as T);
	}

	push_str(s: Str): void {
		this.inner.set((prev) => new String(`${prev}${s}`) as T);
	}

	dehydrate(): { t: "Signal"; id: SignalId } {
		return { t: "Signal", id: this.id };
	}
}
