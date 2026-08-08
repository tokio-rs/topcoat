import { expect, it } from "vitest";

import { Decimal } from "./decimal";

it("equality is scale-insensitive", () => {
	expect(new Decimal("1.5").eq(new Decimal("1.50")).dehydrate()).toBe(true);
	expect(new Decimal("1000").eq(new Decimal("1000.00")).dehydrate()).toBe(true);
	expect(new Decimal("-0").eq(new Decimal("0")).dehydrate()).toBe(true);
	expect(new Decimal("0.0").is_zero().dehydrate()).toBe(true);
});

it("ordering is numeric, not lexicographic", () => {
	// lexicographically "9" > "10"; numerically it is not
	expect(new Decimal("10").gt(new Decimal("9")).dehydrate()).toBe(true);
	expect(new Decimal("1234.50").gt(new Decimal("999.99")).dehydrate()).toBe(
		true,
	);
	expect(new Decimal("0.1").lt(new Decimal("0.11")).dehydrate()).toBe(true);
	expect(new Decimal("-5").lt(new Decimal("-4")).dehydrate()).toBe(true);
	expect(new Decimal("-1").lt(new Decimal("0.5")).dehydrate()).toBe(true);
	expect(new Decimal("-0.01").is_negative().dehydrate()).toBe(true);
});

it("display and round-trip preserve the exact string", () => {
	const d = new Decimal("1234.50");
	expect(d.toNodeText()).toBe("1234.50");
	expect(d.to_string().dehydrate()).toBe("1234.50");
	expect(d.dehydrate()).toEqual({ t: "Decimal", v: "1234.50" });
});

it("matches the Rust surrogate on a shared comparison table", () => {
	// same cases asserted in _decimal.rs, kept in lockstep across the boundary
	const cases: [string, string, "eq" | "lt" | "gt"][] = [
		["1.5", "1.50", "eq"],
		["10", "9", "gt"],
		["0.1", "0.11", "lt"],
		["-5", "-4", "lt"],
		["-1", "0.5", "lt"],
		["1234.50", "999.99", "gt"],
	];
	for (const [a, b, rel] of cases) {
		const da = new Decimal(a);
		const db = new Decimal(b);
		expect(da.eq(db).dehydrate()).toBe(rel === "eq");
		expect(da.lt(db).dehydrate()).toBe(rel === "lt");
		expect(da.gt(db).dehydrate()).toBe(rel === "gt");
	}
});
