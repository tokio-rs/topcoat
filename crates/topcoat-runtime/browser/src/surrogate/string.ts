import { Str } from "./str";

// biome-ignore lint/suspicious/noShadowRestrictedNames: Surrogate type
export class String extends Str {
	deref(): Str {
		return this;
	}

	clone(): String {
		return new String(this.v);
	}

	toJSON(): { t: "String"; v: string } {
		return { t: "String", v: this.v };
	}
}
