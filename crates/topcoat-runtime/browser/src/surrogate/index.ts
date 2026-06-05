import { Event } from "./event";
import { F64 } from "./f64";
import { I32 } from "./i32";
import { Str } from "./str";
// biome-ignore lint/suspicious/noShadowRestrictedNames: Surrogate type
import { String } from "./string";

export * from "./event";
export * from "./f64";
export * from "./i32";
export * from "./ref";
export * from "./signal";
export * from "./str";

export const surrogate = {
	Event(v: globalThis.Event) {
		return new Event(v);
	},
	f64(v: number) {
		return new F64(v);
	},
	i32(v: number) {
		return new I32(v);
	},
	str(v: string) {
		return new Str(v);
	},
	String(v: string) {
		return new String(v);
	},
};
