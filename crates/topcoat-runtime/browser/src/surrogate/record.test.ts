import { expect, it } from "vitest";
import { Context } from "../expression/context";
import { dehydrate } from "../expression/dehydrate";
import { SignalRegistry } from "../signal-registry";
import { F64 } from "./f64";
import type { Option } from "./option";
import { Record } from "./record";
import { Ref } from "./ref";
import { String as RuntimeString } from "./string";

const cx = () => new Context(new SignalRegistry());

it("hydrates records with named fields", () => {
	const wire = {
		t: "Record",
		v: {
			name: "coffee",
			price: 2.5,
			extra: { t: "Option", v: { t: "Record", v: { size: "large" } } },
		},
	};
	const record = cx().hydrate(wire) as Record;
	expect(record).toBeInstanceOf(Record);
	expect(record.name).toBeInstanceOf(RuntimeString);
	expect(record.price).toBeInstanceOf(F64);
	const extra = (record.extra as Option<Record>).unwrap();
	expect(extra).toBeInstanceOf(Record);
	expect(dehydrate(record)).toEqual(wire);
	expect(dehydrate(record.clone())).toEqual(wire);
});

it("rejects malformed record payloads", () => {
	for (const wire of [
		{ t: "Record", v: null },
		{ t: "Record", v: [] },
		{ t: "Record", v: {}, extra: true },
	]) {
		expect(() => cx().hydrate(wire)).toThrow();
	}
});

it("constructs records in expressions", () => {
	const record = cx().record({ name: new RuntimeString("tea") });
	expect(record).toBeInstanceOf(Record);
	expect(dehydrate(record)).toEqual({ t: "Record", v: { name: "tea" } });
});

it("updates records from a base", () => {
	const base = new Record([
		["name", new RuntimeString("tea")],
		["price", new F64(1.5)],
	]);
	const record = cx().recordWith({ price: new F64(2) }, base);
	expect(record).toBeInstanceOf(Record);
	expect(dehydrate(record)).toEqual({
		t: "Record",
		v: { name: "tea", price: 2 },
	});
	expect(dehydrate(base)).toEqual({
		t: "Record",
		v: { name: "tea", price: 1.5 },
	});
});

it("keeps a field named like an object setter as data", () => {
	const record = cx().hydrate(
		JSON.parse('{"t":"Record","v":{"__proto__":1.5}}'),
	) as Record;
	expect(Object.getPrototypeOf(record)).toBe(Record.prototype);
	expect(Object.hasOwn(record, "__proto__")).toBe(true);
});

it("borrows fields of borrowed records", () => {
	const record = new Record([["price", new F64(1.5)]]);
	const reference = Ref.shared(() => record);
	const field = reference.price as Ref<F64>;
	expect(field).toBeInstanceOf(Ref);
	expect(field.deref()).toBe(record.price);
	expect(reference.missing).toBeUndefined();
});

it("clones fields independently of the original", () => {
	const inner = new Record([["size", new RuntimeString("small")]]);
	const record = new Record([["inner", inner]]);
	const copy = record.clone();
	expect(copy.inner).not.toBe(inner);
	expect(dehydrate(copy)).toEqual(dehydrate(record));
});
