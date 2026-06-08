import type { SignalId, SignalRegistry } from "../signal";
import { Bool } from "./bool";
import { F64 } from "./f64";
import { WriteSignal as RuntimeWriteSignal } from "./signal";
import { Str } from "./str";
// biome-ignore lint/suspicious/noShadowRestrictedNames: Surrogate type
import { String } from "./string";

export * from "./bool";
export * from "./event";
export * from "./f64";
export * from "./ref";
export * from "./signal";
export * from "./str";

export type SerializedSurrogate =
	| { t: "bool"; v: boolean }
	| { t: "f64"; v: number }
	| { t: "i32"; v: number }
	| { t: "str"; v: string }
	| { t: "String"; v: string }
	| { t: "signal"; id: SignalId; v?: SerializedSurrogate };

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
		case "signal":
			return new RuntimeWriteSignal(value.id, registry.handle(value.id));
		default:
			throw new Error(`Unknown surrogate type: ${(value as { t: unknown }).t}`);
	}
}
