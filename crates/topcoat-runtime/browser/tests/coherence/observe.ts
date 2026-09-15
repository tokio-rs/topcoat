import { Bool } from "../../src/surrogate/bool";
import { F64 } from "../../src/surrogate/f64";
import { Integer } from "../../src/surrogate/integer";
import { Option } from "../../src/surrogate/option";
import { Ref } from "../../src/surrogate/ref";
import { Result } from "../../src/surrogate/result";
import { Slice } from "../../src/surrogate/sequence";
import { Str } from "../../src/surrogate/string";

export type Value =
	| { type: "Unit" | "None" }
	| { type: "Bool"; value: boolean }
	| { type: "F64" | "String"; value: string }
	| { type: "Integer"; value: { kind: string; bits: number; digits: string } }
	| { type: "Some" | "Ok" | "Err"; value: Value }
	| { type: "Tuple" | "Sequence"; value: Value[] };

/**
 * Reads surrogate storage independently of production dehydration and display.
 * This intentionally depends on the concrete surrogate representations so a
 * lossy serializer cannot make unequal results appear equal.
 */
export function observe(value: unknown): Value {
	if (value === undefined) return { type: "Unit" };
	if (value instanceof Ref) return observe(value.deref());
	if (value instanceof Slice) {
		const items: unknown = Reflect.get(value, "items");
		const start: unknown = Reflect.get(value, "start");
		const end: unknown = Reflect.get(value, "end");
		if (
			!Array.isArray(items) ||
			typeof start !== "number" ||
			typeof end !== "number" ||
			!Number.isInteger(start) ||
			!Number.isInteger(end) ||
			start < 0 || end < start || end > items.length
		) {
			throw new Error("Invalid Slice storage");
		}
		return { type: "Sequence", value: items.slice(start, end).map(observe) };
	}
	if (value instanceof Bool) {
		const stored: unknown = Reflect.get(value, "v");
		if (typeof stored !== "boolean") throw new Error("Invalid Bool storage");
		return { type: "Bool", value: stored };
	}
	if (value instanceof F64) {
		const stored: unknown = Reflect.get(value, "v");
		if (typeof stored !== "number") throw new Error("Invalid F64 storage");
		const bytes = new DataView(new ArrayBuffer(8));
		bytes.setFloat64(0, stored);
		return {
			type: "F64",
			value: Number.isNaN(stored)
				? "nan"
				: bytes.getBigUint64(0).toString(16).padStart(16, "0"),
		};
	}
	if (value instanceof Str) {
		const stored: unknown = Reflect.get(value, "v");
		if (typeof stored !== "string") throw new Error("Invalid Str storage");
		return { type: "String", value: stored };
	}
	if (value instanceof Integer) {
		const stored: unknown = Reflect.get(value, "v");
		const type = Reflect.get(value, "type");
		if (
			typeof stored !== "bigint" ||
			typeof type?.kind !== "string" ||
			typeof type?.bits !== "number"
		) {
			throw new Error("Invalid Integer storage");
		}
		return {
			type: "Integer",
			value: { kind: type.kind, bits: type.bits, digits: stored.toString() },
		};
	}
	if (value instanceof Option) {
		const stored: unknown = Reflect.get(value, "value");
		return stored === undefined
			? { type: "None" }
			: { type: "Some", value: observe(stored) };
	}
	if (value instanceof Result) {
		const kind: unknown = Reflect.get(value, "kind");
		if (kind !== "ok" && kind !== "err") {
			throw new Error("Invalid Result storage");
		}
		return {
			type: kind === "ok" ? "Ok" : "Err",
			value: observe(Reflect.get(value, "value")),
		};
	}
	if (Array.isArray(value)) {
		return { type: "Tuple", value: value.map(observe) };
	}
	throw new Error(
		`Unsupported coherence value: ${Object.prototype.toString.call(value)}`,
	);
}
