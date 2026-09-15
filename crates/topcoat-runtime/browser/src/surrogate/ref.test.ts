import { expect, it } from "vitest";
import { dehydrate } from "../expression/dehydrate";
import { Bool } from "./bool";
import { Integer, integerType } from "./integer";
import { Option } from "./option";
import { cloneValue, Ref } from "./ref";
import { Result } from "./result";
import { String as RuntimeString, Str } from "./string";

it("forwards methods and fields to the current pointee", () => {
	let value = new RuntimeString("first");
	const reference = Ref.shared(() => value);
	expect(reference).toBeInstanceOf(Ref);
	expect(reference.deref()).toBe(value);
	expect(reference.to_owned().toString()).toBe("first");
	expect(reference.toString()).toBe("first");
	value = new RuntimeString("second");
	expect(reference.to_owned().toString()).toBe("second");
	expect(Ref.shared(() => ({ field: value })).field).toBe(value);
});

it("binds methods to their pointee, including private class fields", () => {
	class Value {
		#value = 42;
		get() {
			return this.#value;
		}
	}
	expect(Ref.shared(() => new Value()).get()).toBe(42);
});

it("preserves each level of explicit dereferencing", () => {
	const value = new Integer(42n, integerType("usize", 64));
	const inner = Ref.shared(() => value);
	const outer = Ref.shared(() => inner);
	expect(outer.deref()).toBe(inner);
	expect(outer.deref().deref()).toBe(value);
	expect(outer.increment().toString()).toBe("43");
	expect(outer.clone()).toBe(inner);
});

it("clones owned pointees through methods but preserves container references", () => {
	const value = new RuntimeString("hello");
	const reference = Ref.shared(() => value);
	const cloned = reference.clone();
	expect(cloned).toBeInstanceOf(RuntimeString);
	expect(cloned).not.toBe(value);
	expect(cloned.toString()).toBe("hello");
	expect(cloneValue(reference)).toBe(reference);
	expect(Option.some(reference).clone().unwrap()).toBe(reference);
	expect(Result.from_ok(reference).clone().unwrap()).toBe(reference);
	expect(Result.from_err(reference).clone().unwrap_err()).toBe(reference);
	const str = new Str("borrowed");
	const borrowed = Ref.shared(() => str);
	expect((borrowed as unknown as { clone(): unknown }).clone()).toBe(borrowed);
});

it("keeps dehydration unsupported, even for a serializable pointee", () => {
	const reference = Ref.shared(() => new RuntimeString("hello"));
	expect(() => dehydrate(reference)).toThrow("Ref<T> cannot be dehydrated");
	expect(() => dehydrate(Option.some(reference))).toThrow(
		"Ref<T> cannot be dehydrated",
	);
	expect(dehydrate(reference.to_owned())).toBe("hello");
});

it("rejects mutable dereferencing of a shared reference", () => {
	const value = new RuntimeString("hello");
	expect(() => Ref.shared(() => value).deref_mut()).toThrow("shared Ref");
});

it("does not introduce thenables for unit, booleans, or nested references", async () => {
	for (const value of [undefined, new Bool(true), new RuntimeString("hello")]) {
		const reference = new Ref(() => value);
		expect(await Promise.resolve(reference)).toBe(reference);
		const nested = new Ref(() => reference);
		expect(await Promise.resolve(nested)).toBe(nested);
	}
});
