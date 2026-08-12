import { expect, it } from "vitest";

import { F64 } from "./f64";

// The expected values below are what Rust's `Display` for `f64` produces for
// the same input. An expression runs on the server during the initial render
// and again in the browser, so a surrogate that formats a number differently
// changes what the page says after hydration.
const text = (v: number) => new F64(v).toNodeText();

// Regression for #237: `Number.prototype.toString` switches to exponential
// notation outside 1e-7..1e21. Rust never does.
it("writes large and small magnitudes positionally", () => {
	expect(text(1e21)).toBe("1000000000000000000000");
	expect(text(-1e21)).toBe("-1000000000000000000000");
	expect(text(1e-7)).toBe("0.0000001");
	expect(text(1.2345e-7)).toBe("0.00000012345");
	// Below the switch, both already agree.
	expect(text(1e20)).toBe("100000000000000000000");
	expect(text(1e-6)).toBe("0.000001");
});

it("stays positional at the extremes of the range", () => {
	// 1 followed by 308 zeros, and 324 places after the point.
	expect(text(1e308)).toBe(`1${"0".repeat(308)}`);
	expect(text(5e-324)).toBe(`0.${"0".repeat(323)}5`);
});

it("spells the infinities and NaN like Rust", () => {
	expect(text(Number.POSITIVE_INFINITY)).toBe("inf");
	expect(text(Number.NEGATIVE_INFINITY)).toBe("-inf");
	expect(text(Number.NaN)).toBe("NaN");
});

it("keeps the sign of negative zero", () => {
	expect(text(-0)).toBe("-0");
	expect(text(0)).toBe("0");
});

// Both sides use the shortest digits that round-trip, but when two equally
// short strings are exactly as close to the value, ECMAScript picks the even
// final digit and Rust the one further from zero.
it("breaks an exact digit tie away from zero", () => {
	expect(text(1690060720831323.25)).toBe("1690060720831323.3");
	expect(text(-1006567402717677.25)).toBe("-1006567402717677.3");
	expect(text(233115890514796.125)).toBe("233115890514796.13");
});

it("leaves values with no tie alone", () => {
	expect(text(1.5)).toBe("1.5");
	expect(text(-1.5)).toBe("-1.5");
	expect(text(0.1 + 0.2)).toBe("0.30000000000000004");
	expect(text(1234.5678)).toBe("1234.5678");
	expect(text(1)).toBe("1");
	expect(text(100)).toBe("100");
});

it("formats attribute values the same way", () => {
	expect(new F64(1e21).toAttributeValue()).toBe("1000000000000000000000");
	expect(new F64(-0).toAttributeValue()).toBe("-0");
});
