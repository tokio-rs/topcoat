import { afterEach, expect, it, vi } from "vitest";

import { Effect, flushEffects, signal, untrack } from "./reactivity";

const effects: Effect[] = [];

function watch(callback: () => void): Effect {
	const effect = new Effect(callback);
	effects.push(effect);
	effect.run();
	return effect;
}

afterEach(() => {
	for (const effect of effects) effect.dispose();
	effects.length = 0;
	flushEffects();
});

it("reads writes immediately and batches effects into a microtask", async () => {
	const a = signal(0);
	const b = signal(0);
	const values: number[] = [];
	watch(() => values.push(a() + b()));
	expect(values).toEqual([0]);
	a.set(1);
	a.set((previous) => previous + 1);
	b.set(3);
	expect(a()).toBe(2);
	expect(values).toEqual([0]);
	await Promise.resolve();
	expect(values).toEqual([0, 5]);
});

it("replaces dependencies when an expression takes a different branch", () => {
	const left = signal(true);
	const a = signal(1);
	const b = signal(10);
	const values: number[] = [];
	watch(() => values.push(left() ? a() : b()));
	b.set(11);
	flushEffects();
	expect(values).toEqual([1]);
	left.set(false);
	flushEffects();
	expect(values).toEqual([1, 11]);
	a.set(2);
	flushEffects();
	expect(values).toEqual([1, 11]);
	b.set(12);
	flushEffects();
	expect(values).toEqual([1, 11, 12]);
});

it("untracks nested reads and restores tracking even when they throw", () => {
	const ignored = signal(0);
	const tracked = signal(0);
	const error = new Error("read failed");
	const callback = vi.fn(() => {
		expect(() =>
			untrack(() => {
				untrack(() => ignored());
				throw error;
			}),
		).toThrow(error);
		tracked();
	});
	watch(callback);
	ignored.set(1);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(1);
	tracked.set(1);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(2);
});

it("keeps object identity and !== equality semantics", () => {
	const initial = { value: 1 };
	const value = signal<unknown>(initial);
	const callback = vi.fn(() => value());
	watch(callback);
	value.set(initial);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(1);
	value.set({ value: 1 });
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(2);
	value.set(0);
	flushEffects();
	value.set(-0);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(3);
	value.set(Number.NaN);
	flushEffects();
	value.set(Number.NaN);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(5);
});

it("removes disposed effects from pending and future updates", async () => {
	const value = signal(0);
	const callback = vi.fn(() => value());
	const effect = watch(callback);
	value.set(1);
	effect.dispose();
	effect.dispose();
	await Promise.resolve();
	value.set(2);
	flushEffects();
	effect.run();
	expect(callback).toHaveBeenCalledTimes(1);
});

it("restores the outer observer after a nested effect throws", () => {
	const innerValue = signal(0);
	const outerValue = signal(0);
	const inner = new Effect(() => {
		innerValue();
		throw new Error("inner failed");
	});
	effects.push(inner);
	const callback = vi.fn(() => {
		expect(() => inner.run()).toThrow("inner failed");
		outerValue();
	});
	watch(callback);
	innerValue.set(1);
	expect(flushEffects).toThrow("inner failed");
	expect(callback).toHaveBeenCalledTimes(1);
	outerValue.set(1);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(2);
});

it("processes cascading writes and skips an effect disposed during a flush", () => {
	const a = signal(0);
	const b = signal(0);
	const values: number[] = [];
	watch(() => b.set(a() * 2));
	watch(() => values.push(b()));
	a.set(2);
	flushEffects();
	expect(values).toEqual([0, 4]);

	const trigger = signal(false);
	let child: Effect | undefined;
	watch(() => {
		if (trigger()) child?.dispose();
	});
	const callback = vi.fn(() => trigger());
	child = watch(callback);
	trigger.set(true);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(1);
});

it("notifies other effects without repeatedly scheduling its own writes", () => {
	const value = signal(0);
	const values: number[] = [];
	watch(() => values.push(value()));
	const callback = vi.fn(() => value.set(value() + 1));
	watch(callback);
	flushEffects();
	expect(values).toEqual([0, 1]);
	expect(callback).toHaveBeenCalledTimes(1);
	value.set(5);
	flushEffects();
	expect(value()).toBe(6);
	expect(values.at(-1)).toBe(6);
	expect(callback).toHaveBeenCalledTimes(2);
});

it("finishes other updates after an error and accepts later writes", async () => {
	const value = signal(0);
	const error = new Error("binding failed");
	watch(() => {
		if (value() === 1) throw error;
	});
	const values: number[] = [];
	watch(() => values.push(value()));
	value.set(1);
	expect(flushEffects).toThrow(error);
	expect(values).toEqual([0, 1]);
	value.set(2);
	await Promise.resolve();
	expect(values).toEqual([0, 1, 2]);
});

it("reports all failing effects after draining the queue", () => {
	const value = signal(false);
	const first = new Error("first");
	const second = new Error("second");
	for (const error of [first, second]) {
		watch(() => {
			if (value()) throw error;
		});
	}
	value.set(true);
	let caught: unknown;
	try {
		flushEffects();
	} catch (error) {
		caught = error;
	}
	expect(caught).toBeInstanceOf(AggregateError);
	expect((caught as AggregateError).errors).toEqual([first, second]);
});

it("stops cyclic updates without wedging unrelated effects", () => {
	const a = signal(0);
	const b = signal(0);
	const first = watch(() => b.set(a() + 1));
	const second = watch(() => a.set(b() + 1));
	expect(flushEffects).toThrow("Reactive update cycle exceeded 100 runs");
	first.dispose();
	second.dispose();
	const value = signal(0);
	const values: number[] = [];
	watch(() => values.push(value()));
	value.set(1);
	flushEffects();
	expect(values).toEqual([0, 1]);
});

it("does not duplicate a manual flush when its microtask arrives", async () => {
	const value = signal(0);
	const callback = vi.fn(() => {
		value();
		flushEffects();
	});
	watch(callback);
	value.set(1);
	flushEffects();
	await Promise.resolve();
	expect(callback).toHaveBeenCalledTimes(2);
	value.set(2);
	await Promise.resolve();
	expect(callback).toHaveBeenCalledTimes(3);
});
