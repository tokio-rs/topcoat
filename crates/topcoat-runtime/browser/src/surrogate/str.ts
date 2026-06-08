import { Bool } from "./bool";

export class Str {
	constructor(private readonly v: string) {}

	eq(other: Str): Bool {
		return new Bool(this.v === other.v);
	}

	ne(other: Str): Bool {
		return new Bool(this.v !== other.v);
	}

	gt(other: Str): Bool {
		return new Bool(this.v > other.v);
	}

	lt(other: Str): Bool {
		return new Bool(this.v < other.v);
	}

	ge(other: Str): Bool {
		return new Bool(this.v >= other.v);
	}

	le(other: Str): Bool {
		return new Bool(this.v <= other.v);
	}

	toJSON(): { t: "str"; v: string } {
		return { t: "str", v: this.v };
	}

	toString(): string {
		return this.v.toString();
	}
}
