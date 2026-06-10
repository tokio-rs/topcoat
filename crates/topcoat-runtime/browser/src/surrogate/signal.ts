import type { WriteSignal as MaverickWriteSignal } from "@maverick-js/signals";

import type { SignalId } from "../signal";
import { Ref } from "./ref";

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
		return (this.read().deref() as { clone: () => T }).clone();
	}

	set(v: T): void {
		this.inner.set(v);
	}

	dehydrate(): { t: "Signal"; id: SignalId } {
		return { t: "Signal", id: this.id };
	}
}
