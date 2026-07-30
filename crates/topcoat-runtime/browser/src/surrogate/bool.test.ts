import { expect, it } from "vitest";

import { Bool } from "./bool";

// `expr!` compiles `a && b` to `a.and(() => b)`, so the thunk is the contract:
// evaluating it when the left side already decides the result would change what
// the expression means, not just what it costs.

it("evaluates && to the same value Rust does", () => {
	const table: [boolean, boolean, boolean][] = [
		[true, true, true],
		[true, false, false],
		[false, true, false],
		[false, false, false],
	];

	for (const [a, b, expected] of table) {
		expect(new Bool(a).and(() => new Bool(b)).dehydrate()).toBe(expected);
	}
});

it("evaluates || to the same value Rust does", () => {
	const table: [boolean, boolean, boolean][] = [
		[true, true, true],
		[true, false, true],
		[false, true, true],
		[false, false, false],
	];

	for (const [a, b, expected] of table) {
		expect(new Bool(a).or(() => new Bool(b)).dehydrate()).toBe(expected);
	}
});

it("skips the right side of && when the left side is false", () => {
	let ran = false;
	const right = () => {
		ran = true;
		return new Bool(true);
	};

	expect(new Bool(false).and(right).dehydrate()).toBe(false);
	expect(ran).toBe(false);
});

it("skips the right side of || when the left side is true", () => {
	let ran = false;
	const right = () => {
		ran = true;
		return new Bool(false);
	};

	expect(new Bool(true).or(right).dehydrate()).toBe(true);
	expect(ran).toBe(false);
});

it("still evaluates the right side when it decides the result", () => {
	let ran = false;
	const right = () => {
		ran = true;
		return new Bool(true);
	};

	expect(new Bool(true).and(right).dehydrate()).toBe(true);
	expect(ran).toBe(true);
});
