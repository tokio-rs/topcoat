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

it("counts equal writes separately from ordinary value notifications", () => {
	const initial = {};
	const value = signal<unknown>(initial);
	const values = vi.fn(() => value());
	const revisions: bigint[] = [];
	watch(values);
	watch(() => revisions.push(value.write_revision()));
	for (const next of [initial, {}, 0, -0, Number.NaN, Number.NaN]) {
		value.set(next);
		flushEffects();
	}
	expect(revisions).toEqual([0n, 1n, 2n, 3n, 4n, 5n, 6n]);
	expect(values).toHaveBeenCalledTimes(5);
});

it("preserves the setter's signed-zero return and stored value", () => {
	const value = signal(0);
	expect(Object.is(value.set(-0), 0)).toBe(true);
	expect(Object.is(value(), 0)).toBe(true);
	expect(value.write_revision()).toBe(1n);
});

it("counts nested commits in commit order and keeps them after updater failure", () => {
	const value = signal("A");
	const seen: [string, bigint][] = [];
	watch(() => seen.push([value(), value.write_revision()]));
	expect(
		value.set((previous) => {
			expect(previous).toBe("A");
			value.set("B");
			expect(value.write_revision()).toBe(1n);
			return "A";
		}),
	).toBe("A");
	expect(value.write_revision()).toBe(2n);
	const error = new Error("updater failed");
	expect(() =>
		value.set(() => {
			throw error;
		}),
	).toThrow(error);
	expect(value.write_revision()).toBe(2n);
	expect(() =>
		value.set(() => {
			value.set("C");
			throw error;
		}),
	).toThrow(error);
	expect(value.write_revision()).toBe(3n);
	flushEffects();
	expect(seen).toEqual([
		["A", 0n],
		["C", 3n],
	]);
});

it("batches dual subscriptions and reads coherent state without flushing", async () => {
	const value = signal("A");
	const seen: [bigint, string][] = [];
	watch(() => seen.push([value.write_revision(), value()]));
	value.set("B");
	value.set("A");
	value.set("A");
	expect(value.write_revision()).toBe(3n);
	expect(value()).toBe("A");
	expect(seen).toEqual([[0n, "A"]]);
	await Promise.resolve();
	expect(seen).toEqual([
		[0n, "A"],
		[3n, "A"],
	]);
});

it("replaces revision dependencies with value dependencies on the same handle", () => {
	const revisions = signal(true);
	const value = signal(0);
	const callback = vi.fn(() =>
		revisions() ? value.write_revision() : value(),
	);
	watch(callback);
	value.set(0);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(2);
	revisions.set(false);
	flushEffects();
	value.set(0);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(3);
	value.set(1);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(4);
	revisions.set(true);
	flushEffects();
	value.set(1);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(6);
});

it("untracks revision reads and restores tracking after an error", () => {
	const ignored = signal(0);
	const tracked = signal(0);
	const error = new Error("read failed");
	const callback = vi.fn(() => {
		expect(() =>
			untrack(() => {
				untrack(() => ignored.write_revision());
				throw error;
			}),
		).toThrow(error);
		tracked.write_revision();
	});
	watch(callback);
	ignored.set(0);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(1);
	tracked.set(0);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(2);
});

it("revision self-writes notify other effects without scheduling the running effect", () => {
	const value = signal(0);
	const seen: bigint[] = [];
	watch(() => seen.push(value.write_revision()));
	const callback = vi.fn(() => {
		value.write_revision();
		value.set(0);
	});
	watch(callback);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(1);
	expect(seen).toEqual([0n, 1n]);
	value.set(0);
	flushEffects();
	expect(callback).toHaveBeenCalledTimes(2);
	expect(seen.at(-1)).toBe(3n);
});

it("disposes pending and future revision subscriptions", async () => {
	const value = signal(0);
	const callback = vi.fn(() => value.write_revision());
	const effect = watch(callback);
	value.set(0);
	effect.dispose();
	await Promise.resolve();
	value.set(0);
	flushEffects();
	effect.run();
	expect(callback).toHaveBeenCalledTimes(1);
});

it("drains revision subscribers after errors and recovers for later writes", () => {
	const value = signal(0);
	const errors = [new Error("first"), new Error("second")];
	for (const error of errors) {
		watch(() => {
			if (value.write_revision() === 1n) throw error;
		});
	}
	const seen: bigint[] = [];
	watch(() => seen.push(value.write_revision()));
	value.set(0);
	expect(flushEffects).toThrow(AggregateError);
	expect(seen).toEqual([0n, 1n]);
	value.set(0);
	flushEffects();
	expect(seen).toEqual([0n, 1n, 2n]);
});
