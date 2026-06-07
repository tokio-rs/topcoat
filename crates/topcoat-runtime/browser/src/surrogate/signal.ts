import type { WriteSignal as MaverickWriteSignal } from "@maverick-js/signals";

import type { SignalId } from "../signal";
import { Ref } from "./ref";

export class WriteSignal<T> {
	constructor(
		private readonly id: SignalId,
		private readonly inner: MaverickWriteSignal<Ref<T>>,
	) {}

	read(): Ref<T> {
		return this.inner();
	}

	get(): T {
		return (this.read().deref() as { clone: () => T }).clone();
	}

	set(v: T): void {
		this.inner.set(new Ref(v));
	}

	toJSON(): { t: "signal"; id: SignalId } {
		return { t: "signal", id: this.id };
	}
}
