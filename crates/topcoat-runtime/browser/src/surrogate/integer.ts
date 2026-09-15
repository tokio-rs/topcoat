import type { AttributeValueViewParts, NodeViewParts } from "../dom/view";
import type { IntegerKind, SerializedInteger } from "../expression/serialized";
import { Bool } from "./bool";
import { Panic } from "./panic";

export interface IntegerType {
	readonly kind: IntegerKind;
	readonly bits: number;
	readonly signed: boolean;
	readonly min: bigint;
	readonly max: bigint;
}

/** The width of pointer-sized integers comes from the Rust target. */
export function integerType(kind: IntegerKind, bits: number): IntegerType {
	if (!/^[iu](8|16|32|64|128|size)$/.test(kind)) {
		throw new Error("Unknown integer type");
	}
	const pointer = kind.endsWith("size");
	if (
		(pointer && ![16, 32, 64].includes(bits)) ||
		(!pointer && bits !== Number(kind.slice(1)))
	) {
		throw new Error("Invalid integer width");
	}
	const signed = kind.startsWith("i");
	const limit = 1n << BigInt(bits - Number(signed));
	return Object.freeze({
		kind,
		bits,
		signed,
		min: signed ? -limit : 0n,
		max: limit - 1n,
	});
}

/** Exact, checked arithmetic shared by all Rust integer types. */
export class Integer implements AttributeValueViewParts, NodeViewParts {
	constructor(
		private readonly v: bigint,
		private readonly type: IntegerType,
	) {
		if (v < type.min || v > type.max) {
			throw new Panic("integer overflow");
		}
	}

	static hydrate(value: SerializedInteger): Integer {
		const type = integerType(value.t, value.bits);
		if (
			typeof value.v !== "string" ||
			value.v.length > 40 ||
			!/^(0|-?[1-9][0-9]*)$/.test(value.v)
		) {
			throw new Error("Expected a canonical decimal integer");
		}
		return new Integer(BigInt(value.v), type);
	}

	private operand(other: Integer): bigint {
		if (
			this.type.kind !== other.type.kind ||
			this.type.bits !== other.type.bits
		) {
			throw new Error("Integer types do not match");
		}
		return other.v;
	}

	add(other: Integer): Integer {
		return new Integer(this.v + this.operand(other), this.type);
	}

	sub(other: Integer): Integer {
		return new Integer(this.v - this.operand(other), this.type);
	}

	mul(other: Integer): Integer {
		return new Integer(this.v * this.operand(other), this.type);
	}

	private divisor(other: Integer): bigint {
		const rhs = this.operand(other);
		if (rhs === 0n || (this.v === this.type.min && rhs === -1n)) {
			throw new Panic("invalid integer division or remainder");
		}
		return rhs;
	}

	div(other: Integer): Integer {
		return new Integer(this.v / this.divisor(other), this.type);
	}

	rem(other: Integer): Integer {
		return new Integer(this.v % this.divisor(other), this.type);
	}

	neg(): Integer {
		if (!this.type.signed) throw new Error("Cannot negate an unsigned integer");
		return new Integer(-this.v, this.type);
	}

	eq(other: Integer): Bool {
		return new Bool(this.v === this.operand(other));
	}

	ne(other: Integer): Bool {
		return new Bool(this.v !== this.operand(other));
	}

	gt(other: Integer): Bool {
		return new Bool(this.v > this.operand(other));
	}

	lt(other: Integer): Bool {
		return new Bool(this.v < this.operand(other));
	}

	ge(other: Integer): Bool {
		return new Bool(this.v >= this.operand(other));
	}

	le(other: Integer): Bool {
		return new Bool(this.v <= this.operand(other));
	}

	clone(): Integer {
		return new Integer(this.v, this.type);
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

	toString(): string {
		return this.v.toString();
	}

	dehydrate(): SerializedInteger {
		return { t: this.type.kind, bits: this.type.bits, v: this.v.toString() };
	}
}
