import { Bool } from "../surrogate/bool";
import { F64 } from "../surrogate/f64";
import { Integer, integerType } from "../surrogate/integer";
import { Option } from "../surrogate/option";
import { Procedure } from "../surrogate/procedure";
import { Ref } from "../surrogate/ref";
import { Result } from "../surrogate/result";
import { FixedArray, Slice, Vec } from "../surrogate/sequence";
import { String as RuntimeString, Str } from "../surrogate/string";
import type { Context } from "./context";
import type { DehydratedSurrogate } from "./serialized";

export function hydrate(value: DehydratedSurrogate, cx: Context): unknown {
	if (value === null) return undefined;

	switch (typeof value) {
		case "string":
			return new RuntimeString(value);
		case "number":
			return new F64(value);
		case "boolean":
			return new Bool(value);
		case "bigint":
		case "symbol":
		case "undefined":
		case "function":
			throw new Error(`Unknown surrogate type: ${typeof value}`);
		case "object":
			switch (value.t) {
				case "u8":
				case "u16":
				case "u32":
				case "u64":
				case "u128":
				case "usize":
				case "i8":
				case "i16":
				case "i32":
				case "i64":
				case "i128":
				case "isize":
					return Integer.hydrate(value);
				case "str":
					return new Str(value.v);
				case "Option":
					return value.v === null
						? Option.none()
						: Option.some(hydrate(value.v, cx));
				case "Result":
					return "ok" in value
						? Result.from_ok(hydrate(value.ok, cx))
						: Result.from_err(hydrate(value.err, cx));
				case "Vec":
				case "Array":
				case "Slice": {
					if (
						!Array.isArray(value.v) ||
						Object.keys(value).some((key) => !["t", "bits", "v"].includes(key))
					) {
						throw new Error("Invalid collection payload");
					}
					const type = integerType("usize", value.bits);
					const items = value.v.map((item) => hydrate(item, cx));
					if (value.t === "Vec") return new Vec(items, type);
					if (value.t === "Array") return new FixedArray(items, type);
					const slice = new Slice(items, type);
					return Ref.shared(() => slice);
				}
				case "Signal":
					return cx.signal(value.id);
				case "Procedure":
					return new Procedure(cx, value.path);
				default:
					throw new Error(
						`Unknown surrogate type: ${(value as { t: unknown }).t}`,
					);
			}
	}
}
