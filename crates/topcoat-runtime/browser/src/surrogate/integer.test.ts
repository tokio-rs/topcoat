import { expect, it } from "vitest";
import { Context } from "../expression/context";
import { dehydrate } from "../expression/dehydrate";
import type { IntegerKind } from "../expression/serialized";
import { SignalRegistry } from "../signal-registry";
import { Integer, integerType } from "./integer";
import { Panic } from "./panic";

const types: [IntegerKind, number][] = [
	["u8", 8], ["u16", 16], ["u32", 32], ["u64", 64], ["u128", 128],
	["i8", 8], ["i16", 16], ["i32", 32], ["i64", 64], ["i128", 128],
	["usize", 16], ["usize", 32], ["usize", 64],
	["isize", 16], ["isize", 32], ["isize", 64],
];

for (const [kind, bits] of types) {
	it(`${kind}/${bits} preserves bounds, display, and JSON`, () => {
		const type = integerType(kind, bits);
		const cx = new Context(new SignalRegistry());
		for (const value of [type.min, 0n, 1n, type.max]) {
			const integer = new Integer(value, type);
			const wire = { t: kind, bits, v: value.toString() };
			expect(JSON.parse(JSON.stringify(dehydrate(integer)))).toEqual(wire);
			const hydrated = cx.hydrate(wire) as Integer;
			expect(hydrated.dehydrate()).toEqual(wire);
			expect(hydrated.clone().dehydrate()).toEqual(wire);
			expect(hydrated.toNodeText()).toBe(value.toString());
			expect(hydrated.toAttributeValue()).toBe(value.toString());
			expect(hydrated.isAttributePresent()).toBe(true);
		}
		expect(() => new Integer(type.min - 1n, type)).toThrow(Panic);
		expect(() => new Integer(type.max + 1n, type)).toThrow(Panic);
	});

	it(`${kind}/${bits} checks arithmetic boundaries`, () => {
		const type = integerType(kind, bits);
		const value = (v: bigint) => new Integer(v, type);
		expect(() => value(type.max).add(value(1n))).toThrow(Panic);
		expect(() => value(type.min).sub(value(1n))).toThrow(Panic);
		expect(() => value(type.max).mul(value(2n))).toThrow(Panic);
		expect(() => value(1n).div(value(0n))).toThrow(Panic);
		expect(() => value(1n).rem(value(0n))).toThrow(Panic);
		expect(value(7n).div(value(3n)).toString()).toBe("2");
		expect(value(7n).rem(value(3n)).toString()).toBe("1");
		if (type.signed) {
			expect(() => value(type.min).neg()).toThrow(Panic);
			expect(() => value(type.min).div(value(-1n))).toThrow(Panic);
			expect(() => value(type.min).rem(value(-1n))).toThrow(Panic);
			expect(value(-7n).div(value(3n)).toString()).toBe("-2");
			expect(value(-7n).rem(value(3n)).toString()).toBe("-1");
			expect(value(7n).div(value(-3n)).toString()).toBe("-2");
			expect(value(7n).rem(value(-3n)).toString()).toBe("1");
		}
	});

	it(`${kind}/${bits} signal updates preserve type and reject overflow`, () => {
		const type = integerType(kind, bits);
		const registry = new SignalRegistry();
		registry.insert("count", new Integer(0n, type));
		const signal = new Context(registry).signal("count");
		signal.increment();
		expect((signal.get() as Integer).dehydrate()).toEqual({ t: kind, bits, v: "1" });
		signal.decrement();
		expect((signal.get() as Integer).toString()).toBe("0");
		if (type.signed) {
			signal.decrement();
			expect((signal.get() as Integer).toString()).toBe("-1");
		}
		signal.set(new Integer(type.max, type));
		expect(() => signal.increment()).toThrow(Panic);
		expect((signal.get() as Integer).toString()).toBe(type.max.toString());
		signal.set(new Integer(type.min, type));
		expect(() => signal.decrement()).toThrow(Panic);
		expect((signal.get() as Integer).toString()).toBe(type.min.toString());
	});
}

it("preserves adjacent integers above Number precision", () => {
	const type = integerType("u128", 128);
	const left = new Integer(9007199254740992n, type);
	const right = new Integer(9007199254740993n, type);
	expect(left.eq(right).dehydrate()).toBe(false);
	expect(left.lt(right).dehydrate()).toBe(true);
	expect(right.sub(left).toString()).toBe("1");
});

it("rejects malformed payloads and mismatched types", () => {
	for (const v of ["", "-0", "+1", "01", "1.0", "1e3", " 1", "1 ", "0xff"]) {
		expect(() => Integer.hydrate({ t: "i64", bits: 64, v })).toThrow();
	}
	expect(() => integerType("u8", 64)).toThrow("Invalid integer width");
	expect(() => integerType("usize", 128)).toThrow("Invalid integer width");
	const left = new Integer(1n, integerType("u64", 64));
	const right = new Integer(1n, integerType("usize", 64));
	expect(() => left.add(right)).toThrow("Integer types do not match");
	const narrow = new Integer(1n, integerType("usize", 32));
	expect(() => narrow.eq(right)).toThrow("Integer types do not match");
});

it("updates integer signals without losing precision", () => {
	const registry = new SignalRegistry();
	registry.insert("count", new Integer(9007199254740993n, integerType("usize", 64)));
	const cx = new Context(registry);
	const signal = cx.signal("count");
	const value = signal.get() as Integer;
	signal.set(value.add(new Integer(1n, integerType("usize", 64))));
	expect(dehydrate(signal)).toEqual({
		t: "Signal", id: "count",
		v: { t: "usize", bits: 64, v: "9007199254740994" },
	});
});
