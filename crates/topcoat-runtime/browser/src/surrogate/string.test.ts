import { expect, it } from "vitest";

import { Str, String } from "./string";

// Regression for #192: `String.deref()` returned `this`, so any loop that
// unwraps ref-like values (`while (typeof v.deref === "function")`) never
// terminated, freezing the page on the first client-side text render of an
// owned string.

it("derefs to the borrowed form, not itself", () => {
	const owned = new String("hello");
	const borrowed = owned.deref();

	expect(borrowed).toBeInstanceOf(Str);
	expect(borrowed).not.toBeInstanceOf(String);
	expect(borrowed).not.toBe(owned);
	expect(borrowed.toNodeText()).toBe("hello");
});

it("ref-unwrapping an owned string terminates", () => {
	let current: unknown = new String("hello");
	let steps = 0;
	while (
		current !== null &&
		typeof current === "object" &&
		typeof (current as { deref?: unknown }).deref === "function"
	) {
		current = (current as { deref: () => unknown }).deref();
		expect(++steps).toBeLessThan(10);
	}

	expect(current).toBeInstanceOf(Str);
	expect((current as Str).toNodeText()).toBe("hello");
});
