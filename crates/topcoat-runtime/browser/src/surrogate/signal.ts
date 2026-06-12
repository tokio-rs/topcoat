import {
	type WriteSignal as MaverickWriteSignal,
	signal as maverickSignal,
} from "@maverick-js/signals";
import type { HandleId } from "../scope";
import { Ref } from "./ref";

export class WriteSignal<T> {
	private readonly inner: MaverickWriteSignal<T>;

	constructor(
		private readonly id: HandleId,
		value: T,
	) {
		console.log("constructing signal: ", value);
		this.inner = maverickSignal(value);
	}

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

	dehydrate(): { t: "Signal"; id: HandleId } {
		return { t: "Signal", id: this.id };
	}
}
