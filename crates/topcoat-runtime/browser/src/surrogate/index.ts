import type { Context } from "../context";
import type { SignalId } from "../signal";
import { Bool } from "./bool";
import { F64 } from "./f64";
import { Option } from "./option";
import { Procedure } from "./procedure";
import { Result } from "./result";
import { WriteSignal as RuntimeWriteSignal } from "./signal";
// biome-ignore lint/suspicious/noShadowRestrictedNames: Surrogate type
import { Str, String } from "./string";

export * from "./bool";
export * from "./event";
export * from "./f64";
export * from "./option";
export * from "./panic";
export * from "./procedure";
export * from "./ref";
export * from "./result";
export * from "./signal";

export type DehydratedSurrogate =
	| null
	| boolean
	| number
	| { t: "i32"; v: number }
	| { t: "str"; v: string }
	| string
	| { t: "Option"; v: DehydratedSurrogate | null }
	| { t: "Result"; ok: DehydratedSurrogate }
	| { t: "Result"; err: DehydratedSurrogate }
	| { t: "Signal"; id: SignalId; v?: DehydratedSurrogate }
	| { t: "Procedure"; id: string };

export function hydrateSurrogate(
	value: DehydratedSurrogate,
	cx: Context,
): unknown {
	if (value === null) return undefined;

	switch (typeof value) {
		case "string":
			return new String(value);
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
						: Option.some(hydrateSurrogate(value.v, cx));
				case "Result":
					return "ok" in value
						? Result.from_ok(hydrateSurrogate(value.ok, cx))
						: Result.from_err(hydrateSurrogate(value.err, cx));
				case "Signal":
					return new RuntimeWriteSignal(
						value.id,
						cx.getRegistry().handle(value.id),
					);
				case "Procedure":
					return new Procedure(cx, value.id);
				default:
					throw new Error(
						`Unknown surrogate type: ${(value as { t: unknown }).t}`,
					);
			}
	}
}
