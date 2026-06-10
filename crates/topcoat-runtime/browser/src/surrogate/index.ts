import type { SignalId, SignalRegistry } from "../signal";
import { Action } from "./action";
import { Bool } from "./bool";
import { F64 } from "./f64";
import { Option } from "./option";
import { Result } from "./result";
import { WriteSignal as RuntimeWriteSignal } from "./signal";
// biome-ignore lint/suspicious/noShadowRestrictedNames: Surrogate type
import { Str, String } from "./string";

export * from "./action";
export * from "./bool";
export * from "./event";
export * from "./f64";
export * from "./option";
export * from "./panic";
export * from "./ref";
export * from "./result";
export * from "./signal";

export type SerializedSurrogate =
	| { t: "bool"; v: boolean }
	| { t: "f64"; v: number }
	| { t: "i32"; v: number }
	| { t: "str"; v: string }
	| { t: "String"; v: string }
	| { t: "Option"; v: SerializedSurrogate | null }
	| {
			t: "Result";
			v: { ok: SerializedSurrogate } | { err: SerializedSurrogate };
	  }
	| { t: "signal"; id: SignalId; v?: SerializedSurrogate }
	| { t: "Action"; id: string };

export function deserializeSurrogate(
	value: SerializedSurrogate,
	registry: SignalRegistry,
): unknown {
	switch (value.t) {
		case "bool":
			return new Bool(value.v);
		case "f64":
			return new F64(value.v);
		case "str":
			return new Str(value.v);
		case "String":
			return new String(value.v);
		case "Option":
			return value.v === null
				? Option.none()
				: Option.some(deserializeSurrogate(value.v, registry));
		case "Result":
			return "ok" in value.v
				? Result.from_ok(deserializeSurrogate(value.v.ok, registry))
				: Result.from_err(deserializeSurrogate(value.v.err, registry));
		case "signal":
			return new RuntimeWriteSignal(value.id, registry.handle(value.id));
		case "Action":
			return new Action(value.id);
		default:
			throw new Error(`Unknown surrogate type: ${(value as { t: unknown }).t}`);
	}
}
