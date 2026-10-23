import { expect, it } from "vitest";
import { Context } from "../expression/context";
import { dehydrate } from "../expression/dehydrate";
import { SignalRegistry } from "../signal-registry";
import { Bool } from "./bool";
import { Integer, integerType } from "./integer";
import { Option } from "./option";
import { Panic } from "./panic";
import { Record } from "./record";
import { Ref } from "./ref";
import { type FixedArray, Slice, Vec } from "./sequence";
import { String as RuntimeString } from "./string";

for (const bits of [16, 32, 64]) {
	const type = integerType("usize", bits);
	const index = (value: bigint) => new Integer(value, type);
	for (const kind of ["Vec", "Slice", "Array"] as const) {
		it(`${kind}/${bits} position consumes matches and counts each remaining suffix`, () => {
			const cx = new Context(new SignalRegistry());
			const values = cx.hydrate({
				t: kind,
				bits,
				v: ["a", "b", "b", "c"],
			}) as Slice<RuntimeString>;
			const iter = values.iter();
			const visits: string[] = [];
			const predicate = (value: Ref<RuntimeString> & RuntimeString) => {
				visits.push(value.toString());
				return value.eq(new RuntimeString("b"));
			};
			expect(dehydrate(iter.position(predicate))).toEqual({
				t: "Option",
				v: { t: "usize", bits, v: "1" },
			});
			expect(visits).toEqual(["a", "b"]);
			expect(dehydrate(iter.position(predicate))).toEqual({
				t: "Option",
				v: { t: "usize", bits, v: "0" },
			});
			expect(iter.position(predicate).is_none().dehydrate()).toBe(true);
			expect(iter.position(predicate).is_none().dehydrate()).toBe(true);
			expect(visits).toEqual(["a", "b", "b", "c"]);
			expect(values.iter().position(predicate).unwrap().toString()).toBe("1");
			const empty = cx.hydrate({
				t: kind,
				bits,
				v: [],
			}) as Slice<RuntimeString>;
			expect(
				empty
					.iter()
					.position(() => {
						throw new Error("empty predicate");
					})
					.is_none()
					.dehydrate(),
			).toBe(true);
		});

		it(`${kind}/${bits} hydrates empty collections and checks bounds`, () => {
			const cx = new Context(new SignalRegistry());
			for (const elements of [[], ["first"], ["first", "last"]]) {
				const wire = { t: kind, bits, v: elements };
				const value = cx.hydrate(wire) as Slice<RuntimeString>;
				expect(value.len().dehydrate()).toEqual({
					t: "usize",
					bits,
					v: elements.length.toString(),
				});
				expect(value.is_empty().dehydrate()).toBe(elements.length === 0);
				expect(
					value
						.get(index(BigInt(elements.length)))
						.is_none()
						.dehydrate(),
				).toBe(true);
				expect(value.get(index(type.max)).is_none().dehydrate()).toBe(true);
				if (elements.length > 0) {
					expect(value.index(index(0n)).to_owned().toString()).toBe(
						elements[0],
					);
					const first = value.first().unwrap();
					expect(first).toBeInstanceOf(Ref);
					expect(first.deref().toString()).toBe(elements[0]);
					expect(first.to_owned().toString()).toBe(elements[0]);
					expect(value.last().unwrap().toString()).toBe(elements.at(-1));
				} else {
					expect(() => value.first().unwrap()).toThrow(Panic);
					expect(value.last().is_none().dehydrate()).toBe(true);
				}
				expect(() => value.index(index(BigInt(elements.length)))).toThrow(
					Panic,
				);
				expect(() => value.index(index(type.max))).toThrow(Panic);
				expect(dehydrate(value.to_vec())).toEqual({
					t: "Vec",
					bits,
					v: elements,
				});
				if (kind === "Slice") {
					expect(() => dehydrate(value)).toThrow("Ref<T> cannot be dehydrated");
				} else {
					expect(dehydrate(value)).toEqual(wire);
					expect(dehydrate(value.to_owned())).toEqual(wire);
				}
			}
		});
	}
}

it("rejects malformed collections and indexes of the wrong type or width", () => {
	const cx = new Context(new SignalRegistry());
	for (const wire of [
		{ t: "Vec", bits: 128, v: [] },
		{ t: "Vec", v: [] },
		{ t: "Vec", bits: 64, v: null },
		{ t: "Array", bits: 64, v: {} },
		{ t: "Slice", bits: 64, v: [], extra: true },
	]) {
		expect(() => cx.hydrate(wire)).toThrow();
	}
	const value = new Vec([], integerType("usize", 64));
	for (const type of [
		integerType("usize", 32),
		integerType("u64", 64),
		integerType("i64", 64),
	]) {
		expect(() => value.get(new Integer(0n, type))).toThrow(
			"Index must be a usize",
		);
	}
});

it("position shares elements, retains fixed references, and respects slice offsets", () => {
	const items = ["outside", "first", "last", "outside"].map(
		(value) => new RuntimeString(value),
	);
	const slice = new Slice(items, integerType("usize", 32), 1, 3);
	const iter = slice.iter();
	const visited: (Ref<RuntimeString> & RuntimeString)[] = [];
	expect(
		iter
			.position((value) => {
				visited.push(value);
				return new Bool(true);
			})
			.unwrap()
			.toString(),
	).toBe("0");
	expect(
		iter
			.position((value) => {
				visited.push(value);
				return new Bool(true);
			})
			.unwrap()
			.toString(),
	).toBe("0");
	expect(visited.map((value) => value.deref())).toEqual(items.slice(1, 3));
	expect(visited[0]?.deref()).toBe(items[1]);
	expect(visited[1]?.deref()).toBe(items[2]);
	expect(
		slice
			.iter()
			.position((value) => value.eq(new RuntimeString("last")))
			.unwrap()
			.toString(),
	).toBe("1");
});

it("position propagates failures and resumes after the failing element", () => {
	const items = ["a", "b", "c"].map((value) => new RuntimeString(value));
	for (const failure of [
		new Panic("predicate failed"),
		new Error("foreign failure"),
	]) {
		const iter = new Vec(items, integerType("usize", 64)).iter();
		const visited: string[] = [];
		expect(() =>
			iter.position((value) => {
				visited.push(value.toString());
				if (value.toString() === "b") throw failure;
				return new Bool(false);
			}),
		).toThrow(failure);
		expect(visited).toEqual(["a", "b"]);
		expect(
			iter
				.position((value) => {
					visited.push(value.toString());
					return new Bool(true);
				})
				.unwrap()
				.toString(),
		).toBe("0");
		expect(visited).toEqual(["a", "b", "c"]);
	}
});

it("position uses the sequence width at its maximum representable length", () => {
	const type = integerType("usize", 16);
	const items = Array.from(
		{ length: Number(type.max) },
		(_, i) => new Bool(i === Number(type.max) - 1),
	);
	const slice = new Slice(items, type);
	expect(dehydrate(slice.iter().position((value) => value.clone()))).toEqual({
		t: "Option",
		v: { t: "usize", bits: 16, v: "65534" },
	});
	expect(
		slice
			.iter()
			.position(() => new Bool(false))
			.is_none()
			.dehydrate(),
	).toBe(true);
	expect(() => new Slice([...items, new Bool(false)], type)).toThrow(Panic);
});

it("rejects cursor transport directly and through owned containers and signals", () => {
	const type = integerType("usize", 64);
	const iter = new Vec([new Bool(true)], type).iter();
	const registry = new SignalRegistry();
	registry.insert("cursor", iter);
	for (const value of [
		iter,
		Option.some(iter),
		new Vec([iter], type),
		new Record([["cursor", iter]]),
		new Context(registry).signal("cursor"),
	]) {
		expect(() => dehydrate(value)).toThrow();
	}
});

it("an existing cursor keeps its storage while a fresh signal read sees replacement", () => {
	const type = integerType("usize", 64);
	const registry = new SignalRegistry();
	registry.insert("rows", new Vec([new RuntimeString("old")], type));
	const signal = new Context(registry).signal("rows");
	const read = signal.read() as Ref<Vec<RuntimeString>> & Vec<RuntimeString>;
	const iter = read.iter();
	signal.set(new Vec([new RuntimeString("new")], type));
	expect(
		iter
			.position((value) => value.eq(new RuntimeString("old")))
			.unwrap()
			.toString(),
	).toBe("0");
	expect(
		read
			.iter()
			.position((value) => value.eq(new RuntimeString("new")))
			.unwrap()
			.toString(),
	).toBe("0");
});

it("serializes nested vectors and arrays without losing integer precision", () => {
	const cx = new Context(new SignalRegistry());
	const wire = {
		t: "Vec",
		bits: 64,
		v: [
			{
				t: "Array",
				bits: 64,
				v: [
					{
						t: "u128",
						bits: 128,
						v: "340282366920938463463374607431768211455",
					},
					{ t: "u64", bits: 64, v: "9007199254740993" },
				],
			},
		],
	};
	const value = cx.hydrate(wire) as Vec<FixedArray<Integer>>;
	expect(dehydrate(value)).toEqual(wire);
	expect(dehydrate(value.clone())).toEqual(wire);
	expect(value.first().unwrap().last().unwrap().toString()).toBe(
		"9007199254740993",
	);
});

it("copies owned storage recursively and preserves shared references", () => {
	const type = integerType("usize", 64);
	const text = new RuntimeString("hello");
	const nested = new Vec([new Vec([Option.some(text)], type)], type);
	const copy = nested.clone();
	const originalElement = nested.first().unwrap().deref();
	const copiedElement = copy.first().unwrap().deref();
	expect(copiedElement).not.toBe(originalElement);
	expect(copiedElement.first().unwrap().deref()).not.toBe(
		originalElement.first().unwrap().deref(),
	);
	expect(copiedElement.first().unwrap().unwrap()).not.toBe(text);
	const reference = Ref.shared(() => text);
	const references = new Vec([reference], type);
	expect(references.clone().first().unwrap().deref()).toBe(reference);
	expect(() => dehydrate(references)).toThrow("Ref<T> cannot be dehydrated");
});

it("converts only the visible slice and shares its elements until cloned", () => {
	const type = integerType("usize", 64);
	const elements = ["outside", "inside", "outside"].map(
		(s) => new RuntimeString(s),
	);
	const slice = new Slice(elements, type, 1, 2);
	expect(slice.first().unwrap().deref()).toBe(elements[1]);
	const owned = slice.to_owned();
	expect(dehydrate(owned)).toEqual({ t: "Vec", bits: 64, v: ["inside"] });
	expect(owned.first().unwrap().deref()).not.toBe(elements[1]);
});

it("signal get clones collections while read forwards through a reference", () => {
	const type = integerType("usize", 64);
	const original = new Vec([new RuntimeString("first")], type);
	const registry = new SignalRegistry();
	registry.insert("values", original);
	const signal = new Context(registry).signal("values");
	const copy = signal.get() as Vec<RuntimeString>;
	expect(copy).not.toBe(original);
	expect(copy.first().unwrap().deref()).not.toBe(
		original.first().unwrap().deref(),
	);
	const read = signal.read() as Ref<Vec<RuntimeString>> & Vec<RuntimeString>;
	expect(read.first().unwrap().toString()).toBe("first");
	signal.set(new Vec([new RuntimeString("second")], type));
	expect(read.first().unwrap().toString()).toBe("second");
	expect(copy.first().unwrap().toString()).toBe("first");
	expect(() => dehydrate(read)).toThrow("Ref<T> cannot be dehydrated");
	expect(dehydrate(signal)).toEqual({
		t: "Signal",
		id: "values",
		v: { t: "Vec", bits: 64, v: ["second"] },
	});
});
