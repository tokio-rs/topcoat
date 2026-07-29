import { expect, it } from "vitest";

import { Context } from "../context";
import { SignalRegistry } from "../signal";
import { F64 } from "./f64";
import { hydrateSurrogate } from "./index";
import { Option } from "./option";
import { String as Owned } from "./string";

const cx = () => new Context(new SignalRegistry());

// The inputs below are the JSON `__surrogate` writes for the corresponding
// Rust value, so they pin the wire format rather than a JavaScript shape.

// Regression for the tuple arm being absent: an array fell through the tag
// switch and threw `Unknown surrogate type: undefined`, so every expression
// capturing a tuple failed to hydrate.
it("hydrates a tuple as an array of surrogates", () => {
	// `(1.5f64, 2.5f64)`
	const pair = hydrateSurrogate(JSON.parse("[1.5,2.5]"), cx()) as unknown[];

	expect(Array.isArray(pair)).toBe(true);
	expect(pair).toHaveLength(2);
	expect(pair[0]).toBeInstanceOf(F64);
	// `expr!` compiles `pair.0` to `pair[0]`, so indexing has to work.
	expect((pair[0] as F64).toNodeText()).toBe("1.5");
	expect((pair[1] as F64).toNodeText()).toBe("2.5");
});

it("hydrates the elements of a tuple, not just the tuple", () => {
	// `(1.0f64, Some(2.0f64), "x".to_owned())`
	const parts = hydrateSurrogate(
		JSON.parse('[1.0,{"t":"Option","v":2.0},"x"]'),
		cx(),
	) as unknown[];

	expect(parts[0]).toBeInstanceOf(F64);
	expect(parts[1]).toBeInstanceOf(Option);
	expect((parts[1] as Option<F64>).unwrap()).toBeInstanceOf(F64);
	expect(parts[2]).toBeInstanceOf(Owned);
});

it("hydrates a nested tuple", () => {
	// `((1.0f64, 2.0f64), 3.0f64)`
	const outer = hydrateSurrogate(JSON.parse("[[1.0,2.0],3.0]"), cx()) as [
		unknown[],
		unknown,
	];

	expect(Array.isArray(outer[0])).toBe(true);
	// `2.0f64` renders as `2`, the way Rust's `Display` writes it.
	expect((outer[0][1] as F64).toNodeText()).toBe("2");
	expect(outer[1]).toBeInstanceOf(F64);
});

it("hydrates an empty tuple", () => {
	expect(hydrateSurrogate(JSON.parse("[]"), cx())).toEqual([]);
});
