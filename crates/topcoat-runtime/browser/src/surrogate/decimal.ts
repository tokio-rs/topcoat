import type { AttributeValueViewParts, NodeViewParts } from "../view";
import { Bool } from "./bool";
// biome-ignore lint/suspicious/noShadowRestrictedNames: Surrogate type
import { String } from "./string";

/**
 * An exact decimal number, backed by a validated numeric string. Mirrors the
 * Rust `Decimal` surrogate: compared and displayed as digits, never as a
 * binary float, so money never loses precision at the browser boundary.
 */
export class Decimal implements AttributeValueViewParts, NodeViewParts {
	constructor(private readonly v: string) {}

	clone(): Decimal {
		return new Decimal(this.v);
	}

	eq(other: Decimal): Bool {
		return new Bool(cmp(this.v, other.v) === 0);
	}

	ne(other: Decimal): Bool {
		return new Bool(cmp(this.v, other.v) !== 0);
	}

	gt(other: Decimal): Bool {
		return new Bool(cmp(this.v, other.v) > 0);
	}

	lt(other: Decimal): Bool {
		return new Bool(cmp(this.v, other.v) < 0);
	}

	ge(other: Decimal): Bool {
		return new Bool(cmp(this.v, other.v) >= 0);
	}

	le(other: Decimal): Bool {
		return new Bool(cmp(this.v, other.v) <= 0);
	}

	is_zero(): Bool {
		return new Bool(cmp(this.v, "0") === 0);
	}

	is_negative(): Bool {
		return new Bool(cmp(this.v, "0") < 0);
	}

	to_string(): String {
		return new String(this.v);
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

	dehydrate(): { t: "Decimal"; v: string } {
		return { t: "Decimal", v: this.v };
	}

	toString(): string {
		return this.v;
	}
}

/** Numeric comparison of two validated decimal strings, exact and float-free. */
function cmp(a: string, b: string): number {
	const [aNeg, aMag] = splitSign(a);
	const [bNeg, bMag] = splitSign(b);

	// -0 === 0: a zero magnitude has no sign
	const an = aNeg && !isZeroMag(aMag);
	const bn = bNeg && !isZeroMag(bMag);

	if (an !== bn) return an ? -1 : 1;
	return an ? cmpMag(bMag, aMag) : cmpMag(aMag, bMag);
}

function splitSign(s: string): [boolean, string] {
	return s.startsWith("-") ? [true, s.slice(1)] : [false, s];
}

function isZeroMag(mag: string): boolean {
	for (const ch of mag) if (ch !== "0" && ch !== ".") return false;
	return true;
}

/** Compares two non-negative decimal magnitudes. */
function cmpMag(a: string, b: string): number {
	const [aIntRaw, aFrac] = splitPoint(a);
	const [bIntRaw, bFrac] = splitPoint(b);

	const aInt = stripLeadingZeros(aIntRaw);
	const bInt = stripLeadingZeros(bIntRaw);
	if (aInt.length !== bInt.length) return aInt.length < bInt.length ? -1 : 1;
	if (aInt !== bInt) return aInt < bInt ? -1 : 1;

	const max = Math.max(aFrac.length, bFrac.length);
	for (let i = 0; i < max; i++) {
		const ad = i < aFrac.length ? aFrac.charCodeAt(i) : 48; // '0'
		const bd = i < bFrac.length ? bFrac.charCodeAt(i) : 48;
		if (ad !== bd) return ad < bd ? -1 : 1;
	}
	return 0;
}

function splitPoint(s: string): [string, string] {
	const i = s.indexOf(".");
	return i === -1 ? [s, ""] : [s.slice(0, i), s.slice(i + 1)];
}

function stripLeadingZeros(s: string): string {
	let i = 0;
	while (i < s.length - 1 && s[i] === "0") i++;
	return s.slice(i);
}
