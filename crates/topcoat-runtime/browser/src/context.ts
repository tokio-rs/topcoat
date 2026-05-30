import type { WriteSignal as MaverickWriteSignal } from "@maverick-js/signals";

import type { SignalId, SignalRegistry } from "./signal";

/**
 * The `__context` object passed into every compiled expression. It is the only
 * way generated code can reach back into the runtime — keeping the surface
 * narrow makes the generated JS easy to audit and keeps non-context globals
 * inaccessible from inside `new Function`.
 */
export class Context {
	constructor(private readonly registry: SignalRegistry) {}

	signal(id: SignalId): WriteSignal<unknown> {
		return new WriteSignal(this.registry.handle(id));
	}

	get builtin() {
		return builtin;
	}
}

const builtin = {
	f64(v: number): f64 {
		return new f64(v);
	},
};

export class f64 {
	constructor(private readonly v: number) {}

	add(other: f64): f64 {
		return new f64(this.v + other.v);
	}

	sub(other: f64): f64 {
		return new f64(this.v - other.v);
	}

	mul(other: f64): f64 {
		return new f64(this.v * other.v);
	}

	div(other: f64): f64 {
		return new f64(this.v / other.v);
	}

	toString(): string {
		return this.v.toString();
	}
}

export class Ref<T> {
	constructor(private readonly pointee: T) {}

	deref(): T {
		return this.pointee;
	}
}

class WriteSignal<T> {
	constructor(private readonly inner: MaverickWriteSignal<Ref<T>>) {}

	read(): Ref<T> {
		return this.inner();
	}

	set(v: T): void {
		this.inner.set(new Ref(v));
	}
}
