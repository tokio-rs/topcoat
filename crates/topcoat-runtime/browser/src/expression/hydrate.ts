import { Bool } from "../surrogate/bool";
import { F64 } from "../surrogate/f64";
import { Option } from "../surrogate/option";
import { Procedure } from "../surrogate/procedure";
import { Result } from "../surrogate/result";
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
				case "Signal":
					return cx.signal(value.id);
				case "Procedure":
					return new Procedure(cx, value.id);
				default:
					throw new Error(
						`Unknown surrogate type: ${(value as { t: unknown }).t}`,
					);
			}
	}
}
