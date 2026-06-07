import type { SignalId, SignalRegistry } from "../signal";
import { F64 } from "./f64";
import { I32 } from "./i32";
import { WriteSignal as RuntimeWriteSignal } from "./signal";
import { Str } from "./str";
// biome-ignore lint/suspicious/noShadowRestrictedNames: Surrogate type
import { String } from "./string";

export * from "./event";
export * from "./f64";
export * from "./i32";
export * from "./ref";
export * from "./signal";
export * from "./str";

export type SerializedSurrogate =
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
		case "f64":
			return new F64(value.v);
		case "i32":
			return new I32(value.v);
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
