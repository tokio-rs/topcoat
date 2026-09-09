import { effect, root, signal, tick } from "@maverick-js/signals";
import { expect, it } from "vitest";

import { Bool } from "./bool";
import { F64 } from "./f64";
import { WriteSignal } from "./signal";
import { Str, String } from "./string";

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
	const before = new String("hi");
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
	const s = new WriteSignal("abc", signal<unknown>(new String("shoes")));

	expect(s.dehydrate()).toEqual({ t: "Signal", id: "abc", v: "shoes" });

	s.set(new String("boots"));
	expect(s.dehydrate()).toEqual({ t: "Signal", id: "abc", v: "boots" });
});

// Each write must construct a new value rather than mutate the stored one:
// change detection is identity based, so a future refactor that mutates in
// place would silently stop notifying subscribers.
it("each write notifies subscribers exactly once", () => {
	root((dispose) => {
		const inner = signal<unknown>(new F64(0));
		const s = new WriteSignal("test", inner);

		let runs = 0;
		effect(() => {
			inner();
			runs += 1;
		});
		tick();
		expect(runs).toBe(1);

		s.increment();
		tick();
		expect(runs).toBe(2);

		s.decrement();
		tick();
		expect(runs).toBe(3);

		dispose();
	});
});
