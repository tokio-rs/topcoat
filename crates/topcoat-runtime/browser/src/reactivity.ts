let activeEffect: Effect | null = null;
const pending = new Set<Effect>();
let scheduled = false;
let flushing = false;

export interface WriteSignal<T> {
	(): T;
	set(value: T | ((previous: T) => T)): T;
	write_revision(): bigint;
}

/** A value whose reads subscribe the running effect to future changes. */
export function signal<T>(initial: T): WriteSignal<T> {
	return createSignal(initial, 0n);
}

// The initial revision is private so boundary tests need no public seed API.
function createSignal<T>(initial: T, revision: bigint): WriteSignal<T> {
	let value = initial;
	const subscribers = new Set<Effect>();
	const revisionSubscribers = new Set<Effect>();
	const read = () => {
		activeEffect?.track(subscribers);
		return value;
	};
	read.set = (next: T | ((previous: T) => T)): T => {
		const updated =
			typeof next === "function" ? (next as (previous: T) => T)(value) : next;
		// Check after the updater: nested writes have already committed. Its
		// side effects remain even if this commit fails.
		if (revision === 0xffff_ffff_ffff_ffffn) {
			throw new Error("Signal write revision overflow");
		}
		revision += 1n;
		// Preserve reference equality for surrogate objects, and !== semantics
		// for primitives (including NaN and signed zero).
		if (value !== updated) {
			value = updated;
			for (const effect of subscribers) effect.schedule();
		}
		for (const effect of revisionSubscribers) effect.schedule();
		return value;
	};
	read.write_revision = (): bigint => {
		activeEffect?.track(revisionSubscribers);
		return revision;
	};
	return read;
}

/** Reads inside `fn` do not become dependencies of the running effect. */
export function untrack<T>(fn: () => T): T {
	const previous = activeEffect;
	activeEffect = null;
	try {
		return fn();
	} finally {
		activeEffect = previous;
	}
}

/**
 * A synchronous reaction that tracks its signal reads. Its owner must dispose
 * it when no longer needed. Creating an effect inside another does not link
 * their lifetimes.
 */
export class Effect {
	private readonly dependencies = new Set<Set<Effect>>();
	private disposed = false;
	private running = false;

	constructor(private readonly callback: () => void) {}

	track(subscribers: Set<Effect>): void {
		if (this.disposed) return;
		subscribers.add(this);
		this.dependencies.add(subscribers);
	}

	schedule(): void {
		// Writes during an effect notify other subscribers but do not schedule
		// the running effect again.
		if (this.disposed || this.running) return;
		pending.add(this);
		if (!scheduled && !flushing) {
			scheduled = true;
			queueMicrotask(() => {
				scheduled = false;
				flushEffects();
			});
		}
	}

	run(): void {
		if (this.disposed || this.running) return;
		pending.delete(this);
		this.unsubscribe();
		const previous = activeEffect;
		activeEffect = this;
		this.running = true;
		try {
			this.callback();
		} finally {
			this.running = false;
			activeEffect = previous;
		}
	}

	dispose(): void {
		this.disposed = true;
		pending.delete(this);
		this.unsubscribe();
	}

	private unsubscribe(): void {
		for (const subscribers of this.dependencies) subscribers.delete(this);
		this.dependencies.clear();
	}
}

/** Drains scheduled updates synchronously; also used by the microtask queue. */
export function flushEffects(): void {
	if (flushing) return;
	flushing = true;
	const errors: unknown[] = [];
	const runs = new Map<Effect, number>();
	try {
		// Removing each entry before running it allows writes from other
		// effects to enqueue it again, while repeated writes stay batched.
		for (const effect of pending) {
			pending.delete(effect);
			const count = (runs.get(effect) ?? 0) + 1;
			runs.set(effect, count);
			if (count > 100) {
				errors.push(new Error("Reactive update cycle exceeded 100 runs"));
				continue;
			}
			try {
				effect.run();
			} catch (error) {
				errors.push(error);
			}
		}
	} finally {
		flushing = false;
	}
	// One broken binding must not prevent unrelated bindings from updating
	// or leave the scheduler stuck. Report errors after draining the queue.
	if (errors.length === 1) throw errors[0];
	if (errors.length > 1)
		throw new AggregateError(errors, "Reactive updates failed");
}

// In-source tests can seed the private counter without shipping a reset API.
if (import.meta.vitest) {
	const { it, expect } = import.meta.vitest;

	it("rejects overflow without committing or notifying either dependency", () => {
		const maximum = (1n << 64n) - 1n;
		const value = createSignal(0, maximum - 1n);
		const seen: [number, bigint][] = [];
		const effect = new Effect(() =>
			seen.push([value(), value.write_revision()]),
		);
		try {
			effect.run();
			value.set(1);
			flushEffects();
			expect(value.write_revision()).toBe(maximum);
			expect(() => value.set(2)).toThrow();
			let sideEffect = false;
			expect(() =>
				value.set(() => {
					sideEffect = true;
					return 3;
				}),
			).toThrow();
			flushEffects();
			expect(sideEffect).toBe(true);
			expect(seen).toEqual([
				[0, maximum - 1n],
				[1, maximum],
			]);
			expect(value()).toBe(1);
			expect(value.write_revision()).toBe(maximum);
		} finally {
			effect.dispose();
		}
	});

	it("checks overflow after a nested commit reaches the limit", () => {
		const maximum = (1n << 64n) - 1n;
		const value = createSignal(0, maximum - 1n);
		expect(() =>
			value.set(() => {
				value.set(1);
				return 2;
			}),
		).toThrow();
		expect(value()).toBe(1);
		expect(value.write_revision()).toBe(maximum);
	});

	it("counts exactly beyond Number's integer precision", () => {
		const value = createSignal(0, 1n << 53n);
		value.set(0);
		expect(value.write_revision()).toBe((1n << 53n) + 1n);
	});
}
