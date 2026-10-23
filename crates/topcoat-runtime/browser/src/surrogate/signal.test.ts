import { expect, it } from "vitest";

import { Effect, flushEffects, signal } from "../reactivity";
import { Bool } from "./bool";
import { F64 } from "./f64";
import { Integer, integerType } from "./integer";
import { WriteSignal } from "./signal";
import { String as RuntimeString, Str } from "./string";

function write<T>(value: T): WriteSignal<T> {
	return new WriteSignal("test", signal(value));
}

it("toggle flips a boolean and flips it back", () => {
	const s = write(new Bool(false));

	s.toggle();
	expect(s.get().dehydrate()).toBe(true);

	s.toggle();
	expect(s.get().dehydrate()).toBe(false);
});

it("increment and decrement move by one, including across zero", () => {
	const s = write(new F64(0));

	s.increment();
	expect(s.get().dehydrate()).toBe(1);

	s.decrement();
	s.decrement();
	expect(s.get().dehydrate()).toBe(-1);
});

it("push_str appends and leaves the previous value untouched", () => {
	const before = new RuntimeString("hi");
	const s = write(before);

	s.push_str(new Str("!"));

	expect(s.get().dehydrate()).toBe("hi!");
	expect(before.dehydrate()).toBe("hi");
});

// A signal passed to a shard travels as its id and current value, so the
// server can rebuild it: the value alone would lose the identity a tracked
// read inside the shard depends on, and the id alone would leave the server
// nothing to read.
it("dehydrates to its id and current value", () => {
	const s = new WriteSignal("abc", signal<unknown>(new RuntimeString("shoes")));

	expect(s.dehydrate()).toEqual({ t: "Signal", id: "abc", v: "shoes" });

	s.set(new RuntimeString("boots"));
	expect(s.dehydrate()).toEqual({ t: "Signal", id: "abc", v: "boots" });
});

// Each write must construct a new value rather than mutate the stored one:
// change detection is identity based, so a future refactor that mutates in
// place would silently stop notifying subscribers.
it("each write notifies subscribers exactly once", () => {
	const inner = signal<unknown>(new F64(0));
	const s = new WriteSignal("test", inner);

	let runs = 0;
	const effect = new Effect(() => {
		inner();
		runs += 1;
	});
	try {
		effect.run();
		expect(runs).toBe(1);

		s.increment();
		flushEffects();
		expect(runs).toBe(2);

		s.decrement();
		flushEffects();
		expect(runs).toBe(3);
	} finally {
		effect.dispose();
	}
});

it("aliases share one u64 revision without changing serialization or reads", () => {
	const inner = signal(new Bool(false));
	const first = new WriteSignal("same", inner);
	const second = new WriteSignal("same", inner);
	const independent = write(new Bool(false));
	const before = first.write_revision();
	first.set(new Bool(false));
	second.read().deref_mut();
	first.get();
	first.read();
	expect(first.dehydrate()).toEqual({ t: "Signal", id: "same", v: false });
	expect(before.dehydrate()).toEqual({ t: "u64", bits: 64, v: "0" });
	expect(second.write_revision().dehydrate()).toEqual({
		t: "u64",
		bits: 64,
		v: "2",
	});
	expect(independent.write_revision().dehydrate()).toEqual({
		t: "u64",
		bits: 64,
		v: "0",
	});
});

it("every surrogate mutation commits once, including no-op writes through Ref", () => {
	const bool = write(new Bool(false));
	const number = write(new F64(0));
	const integer = write(new Integer(0n, integerType("u64", 64)));
	const text = write(new RuntimeString(""));
	for (const [s, mutate] of [
		[bool, () => bool.toggle()],
		[number, () => number.increment()],
		[number, () => number.decrement()],
		[integer, () => integer.increment()],
		[integer, () => integer.decrement()],
		[text, () => text.push_str(new Str(""))],
		[text, () => text.push_str(new Str("hello"))],
		[text, () => text.read().deref_mut()],
	] as const) {
		const before = s.write_revision();
		mutate();
		expect(s.write_revision().eq(before.increment()).dehydrate()).toBe(true);
	}
});

it("failed mutation helpers do not commit or schedule revision subscribers", () => {
	const s = write(new Integer(0n, integerType("u8", 8)));
	const revisions: string[] = [];
	const effect = new Effect(() =>
		revisions.push(s.write_revision().toString()),
	);
	try {
		effect.run();
		expect(() => s.decrement()).toThrow();
		flushEffects();
		expect(revisions).toEqual(["0"]);
		expect(s.get().toString()).toBe("0");
		s.increment();
		flushEffects();
		expect(revisions).toEqual(["0", "1"]);
	} finally {
		effect.dispose();
	}
});

it("same-surrogate writes notify only revisions while equal new surrogates keep value notifications", () => {
	const initial = new Bool(false);
	const s = write(initial);
	let valueRuns = 0;
	const seen: string[] = [];
	const valueEffect = new Effect(() => {
		s.read().deref();
		valueRuns += 1;
	});
	const revisionEffect = new Effect(() =>
		seen.push(s.write_revision().toString()),
	);
	try {
		valueEffect.run();
		revisionEffect.run();
		s.set(initial);
		flushEffects();
		expect(valueRuns).toBe(1);
		s.set(new Bool(false));
		flushEffects();
		expect(valueRuns).toBe(2);
		expect(seen).toEqual(["0", "1", "2"]);
	} finally {
		valueEffect.dispose();
		revisionEffect.dispose();
	}
});
