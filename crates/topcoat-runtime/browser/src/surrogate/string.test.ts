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

// The expected values below are what Rust produces for the same input. An
// expression runs on the server during the initial render and again in the
// browser, so a surrogate that disagrees with `str` changes what the page
// says after hydration.

// Regression for #126: `len()` counted UTF-16 code units.
it("counts length in UTF-8 bytes", () => {
	expect(new Str("한😊").len().toString()).toBe("7");
	expect(new Str("a").len().toString()).toBe("1");
	expect(new Str("é").len().toString()).toBe("2");
});

// Regression for #236: `<` and friends order by UTF-16 code unit, which puts
// U+1F600 below U+FB00 because its high surrogate is 0xD83D.
it("orders strings by code point", () => {
	const below = new Str("\u{FB00}");
	const above = new Str("\u{1F600}");

	expect(below.lt(above).toString()).toBe("true");
	expect(below.le(above).toString()).toBe("true");
	expect(above.gt(below).toString()).toBe("true");
	expect(above.ge(below).toString()).toBe("true");
});

it("orders a prefix before the string that extends it", () => {
	const short = new Str("ab");
	const long = new Str("abc");

	expect(short.lt(long).toString()).toBe("true");
	expect(long.gt(short).toString()).toBe("true");
	expect(short.le(new Str("ab")).toString()).toBe("true");
	expect(short.ge(new Str("ab")).toString()).toBe("true");
	expect(short.gt(new Str("ab")).toString()).toBe("false");
});

// Regression for #238: the ECMAScript whitespace set strips U+FEFF and keeps
// U+0085, and Rust's `White_Space` property does the opposite.
it("trims the code points Rust treats as whitespace", () => {
	expect(new Str("\u{FEFF}x\u{FEFF}").trim().toNodeText()).toBe(
		"\u{FEFF}x\u{FEFF}",
	);
	expect(new Str("\u{0085}x\u{0085}").trim().toNodeText()).toBe("x");
	expect(new Str("\u{00A0}\u{1680}\u{2000}\u{3000}x").trim().toNodeText()).toBe(
		"x",
	);
	expect(new Str("\u{180E}x").trim().toNodeText()).toBe("\u{180E}x");
	expect(new Str(" \t\n x \r\n ").trim().toNodeText()).toBe("x");
});

it("trims only the requested end", () => {
	expect(new Str("\u{0085}x\u{0085}").trim_start().toNodeText()).toBe(
		"x\u{0085}",
	);
	expect(new Str("\u{0085}x\u{0085}").trim_end().toNodeText()).toBe(
		"\u{0085}x",
	);
	expect(new Str("x\u{FEFF}").trim_end().toNodeText()).toBe("x\u{FEFF}");
});
