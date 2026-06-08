import { Bool } from "./bool";

// biome-ignore lint/suspicious/noShadowRestrictedNames: Surrogate type
export class String {
	constructor(private readonly v: string) {}

	clone(): String {
		return new String(this.v);
	}

	eq(other: String): Bool {
		return new Bool(this.v === other.v);
	}

	ne(other: String): Bool {
		return new Bool(this.v !== other.v);
	}

	gt(other: String): Bool {
		return new Bool(this.v > other.v);
	}

	lt(other: String): Bool {
		return new Bool(this.v < other.v);
	}

	ge(other: String): Bool {
		return new Bool(this.v >= other.v);
	}

	le(other: String): Bool {
		return new Bool(this.v <= other.v);
	}

	toJSON(): { t: "String"; v: string } {
		return { t: "String", v: this.v };
	}

	toString(): string {
		return this.v.toString();
	}
}
