import type { AttributeValueViewParts, NodeViewParts } from "../view";
import { Bool } from "./bool";
import { F64 } from "./f64";

export class Str implements AttributeValueViewParts, NodeViewParts {
	constructor(protected readonly v: string) {}

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

	is_empty(): Bool {
		return new Bool(this.v.length === 0);
	}

	len(): F64 {
		return new F64(this.v.length);
	}

	trim(): Str {
		return new Str(this.v.trim());
	}

	trim_start(): Str {
		return new Str(this.v.trimStart());
	}

	trim_end(): Str {
		return new Str(this.v.trimEnd());
	}

	starts_with(other: Str): Bool {
		return new Bool(this.v.startsWith(other.v));
	}

	ends_with(other: Str): Bool {
		return new Bool(this.v.endsWith(other.v));
	}

	contains(other: Str): Bool {
		return new Bool(this.v.includes(other.v));
	}

	isAttributePresent(): boolean {
		return true;
	}

	toAttributeValue(): string {
		return this.v;
	}

	toNodeText(): string {
		return this.v;
	}

	toJSON(): { t: "str" | "String"; v: string } {
		return { t: "str", v: this.v };
	}

	toString(): string {
		return this.v.toString();
	}
}
