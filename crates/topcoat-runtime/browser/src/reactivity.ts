let activeEffect: Effect | null = null;
const pending = new Set<Effect>();
let scheduled = false;
let flushing = false;

/**
 * A reactive value. Calling it reads the value, and `set` replaces it with a
 * new value or with the result of a function of the previous one.
 */
export interface WriteSignal<T> {
	(): T;
	set(value: T | ((previous: T) => T)): T;
}

/** A value whose reads subscribe the running effect to future changes. */
export function signal<T>(initial: T): WriteSignal<T> {
	let value = initial;
	const subscribers = new Set<Effect>();
	const read = () => {
		activeEffect?.track(subscribers);
		return value;
	};
	read.set = (next: T | ((previous: T) => T)): T => {
		const updated =
			typeof next === "function" ? (next as (previous: T) => T)(value) : next;
		// Preserve reference equality for surrogate objects, and !== semantics
		// for primitives (including NaN and signed zero).
		if (value !== updated) {
			value = updated;
			for (const effect of subscribers) effect.schedule();
		}
		return value;
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
 * A synchronous reaction with explicit ownership. Scope registers it before
 * its first run and disposes it when the content is released. There is no
 * implicit effect tree: creating another effect does not make it a child.
 */
export class Effect {
	private readonly dependencies = new Set<Set<Effect>>();
	private disposed = false;
	private running = false;

	constructor(private readonly callback: () => void) {}

	/** Subscribes this effect to a signal's subscriber set. */
	track(subscribers: Set<Effect>): void {
		if (this.disposed) return;
		subscribers.add(this);
		this.dependencies.add(subscribers);
	}

	/** Queues this effect to run in a microtask. */
	schedule(): void {
		// Writes during an effect do not schedule that same effect again. They
		// still notify its other subscribers.
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

	/** Runs the callback now, tracking the signals it reads. */
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

	/** Stops this effect and drops its subscriptions. */
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
