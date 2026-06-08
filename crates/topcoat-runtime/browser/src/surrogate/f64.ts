import type { AttributeValueViewParts, NodeViewParts } from "../view";
import { Bool } from "./bool";

export class F64 implements AttributeValueViewParts, NodeViewParts {
	constructor(private readonly v: number) {}

	add(other: F64): F64 {
		return new F64(this.v + other.v);
	}

	sub(other: F64): F64 {
		return new F64(this.v - other.v);
	}

	mul(other: F64): F64 {
		return new F64(this.v * other.v);
	}

	div(other: F64): F64 {
		return new F64(this.v / other.v);
	}

	neg(): F64 {
		return new F64(-this.v);
	}

	eq(other: F64): Bool {
		return new Bool(this.v === other.v);
	}

	ne(other: F64): Bool {
		return new Bool(this.v !== other.v);
	}

	gt(other: F64): Bool {
		return new Bool(this.v > other.v);
	}

	lt(other: F64): Bool {
		return new Bool(this.v < other.v);
	}

	ge(other: F64): Bool {
		return new Bool(this.v >= other.v);
	}

	le(other: F64): Bool {
		return new Bool(this.v <= other.v);
	}

	clone(): F64 {
		return new F64(this.v);
	}

	isAttributePresent(): boolean {
		return true;
	}

	toAttributeValue(): string {
		return this.v.toString();
	}

	toNodeText(): string {
		return this.v.toString();
	}

	toJSON(): { t: "f64"; v: number } {
		return { t: "f64", v: this.v };
	}

	toString(): string {
		return this.v.toString();
	}

	valueOf(): number {
		return this.v;
	}
}
