import type { AttributeValueViewParts, NodeViewParts } from "../view";
import { Bool } from "./bool";

const BITS = new DataView(new ArrayBuffer(8));

// `Math.abs(v)` as the exact `mantissa * 2 ** exponent` it is stored as.
function decompose(v: number): { mantissa: bigint; exponent: number } {
	BITS.setFloat64(0, Math.abs(v));
	const raw = BITS.getBigUint64(0);
	const biased = Number((raw >> 52n) & 0x7ffn);
	const fraction = raw & 0xf_ffff_ffff_ffffn;
	// A biased exponent of 0 is subnormal: no implicit leading bit.
	return biased === 0
		? { mantissa: fraction, exponent: -1074 }
		: { mantissa: fraction | 0x10_0000_0000_0000n, exponent: biased - 1075 };
}

// Whether `v` sits exactly halfway between `digits * 10 ** scale` and the next
// digit string up, so both are equally short and equally close. ECMAScript
// resolves that tie to the even final digit and Rust to the value further from
// zero, which is the only case where the two disagree on which digits to use.
function tiedBelow(v: number, digits: string, scale: number): boolean {
	// ECMAScript only lands on an even final digit by resolving a tie, so an
	// odd one rules the tie out before doing any exact arithmetic.
	if (Number(digits[digits.length - 1]) % 2 !== 0) return false;
	const { mantissa, exponent } = decompose(v);
	// 2 * mantissa * 2**exponent === (2 * digits + 1) * 10**scale, with both
	// sides multiplied up until every exponent is non-negative.
	const twos = exponent < 0 ? -exponent : 0;
	const tens = scale < 0 ? -scale : 0;
	const left =
		2n * mantissa * 2n ** BigInt(exponent + twos) * 10n ** BigInt(tens);
	const right =
		(2n * BigInt(digits) + 1n) *
		10n ** BigInt(scale + tens) *
		2n ** BigInt(twos);
	return left === right;
}

// Render a number the way Rust's `Display` for `f64` does, which is how the
// server renders it (`ViewPart::F64`). `Number.prototype.toString` differs on
// five kinds of value (#237):
//
// - Outside 1e-7..1e21 it switches to exponential notation. Rust never does:
//   it writes `1e21` as 22 digits and `5e-324` as 326 characters.
// - It spells the infinities `Infinity` and `-Infinity`, Rust `inf` and `-inf`.
// - It drops the sign of negative zero, Rust keeps it.
// - On an exact tie between two equally short digit strings it picks the even
//   final digit, Rust the value further from zero.
//
// Otherwise both pick the same shortest digits that round-trip, so the rest is
// moving the decimal point.
export function display(v: number): string {
	if (Number.isNaN(v)) return "NaN";
	if (v === Number.POSITIVE_INFINITY) return "inf";
	if (v === Number.NEGATIVE_INFINITY) return "-inf";
	if (v === 0) return Object.is(v, -0) ? "-0" : "0";

	// `[-]d[.ddd]e[+-]k`, always the shortest digits that round-trip.
	const exponential = v.toExponential();
	const marker = exponential.indexOf("e");
	const mantissa = exponential.slice(0, marker);
	const sign = mantissa.startsWith("-") ? "-" : "";
	let digits = (sign === "" ? mantissa : mantissa.slice(1)).replace(".", "");
	const scale = Number(exponential.slice(marker + 1)) - (digits.length - 1);

	if (tiedBelow(v, digits, scale)) {
		// An even final digit never carries, so the length is unchanged.
		digits = (BigInt(digits) + 1n).toString();
	}

	const point = digits.length + scale;
	if (point <= 0) return `${sign}0.${"0".repeat(-point)}${digits}`;
	if (point >= digits.length) {
		return sign + digits + "0".repeat(point - digits.length);
	}
	return `${sign}${digits.slice(0, point)}.${digits.slice(point)}`;
}

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
		return display(this.v);
	}

	toNodeText(): string {
		return display(this.v);
	}

	dehydrate(): number {
		return this.v;
	}

	toString(): string {
		return display(this.v);
	}
}
