import type { AttributeValueViewParts, NodeViewParts } from "../view";
import { Bool } from "./bool";
import { F64 } from "./f64";

const TEXT_ENCODER = new TextEncoder();

// The code points Rust's `str::trim` family treats as whitespace: the Unicode
// `White_Space` property. `String.prototype.trim` uses the ECMAScript
// whitespace set instead, which strips U+FEFF and keeps U+0085 -- both the
// opposite of Rust (#238).
const WHITE_SPACE =
	"\\t\\n\\v\\f\\r \\u0085\\u00A0\\u1680\\u2000-\\u200A\\u2028\\u2029\\u202F\\u205F\\u3000";
const TRIM_START = new RegExp(`^[${WHITE_SPACE}]+`, "u");
const TRIM_END = new RegExp(`[${WHITE_SPACE}]+$`, "u");

// Order two strings the way Rust's `str` does: by code point, which is the
// order of their UTF-8 bytes. JavaScript's relational operators order by
// UTF-16 code unit, so every code point above U+FFFF sorts below U+E000 --
// its high surrogate is 0xD800-0xDBFF -- and the two sides of one `$()`
// expression can disagree (#236).
function compare(a: string, b: string): number {
	const left = a[Symbol.iterator]();
	const right = b[Symbol.iterator]();
	for (;;) {
		const x = left.next();
		const y = right.next();
		// A string that ran out is a prefix of the other, so it sorts first.
		if (x.done || y.done) return Number(y.done) - Number(x.done);
		if (x.value !== y.value) {
			return (
				(x.value.codePointAt(0) as number) - (y.value.codePointAt(0) as number)
			);
		}
	}
}

export class Str implements AttributeValueViewParts, NodeViewParts {
	constructor(protected readonly v: string) {}

	eq(other: Str): Bool {
		return new Bool(this.v === other.v);
	}

	ne(other: Str): Bool {
		return new Bool(this.v !== other.v);
	}

	gt(other: Str): Bool {
		return new Bool(compare(this.v, other.v) > 0);
	}

	lt(other: Str): Bool {
		return new Bool(compare(this.v, other.v) < 0);
	}

	ge(other: Str): Bool {
		return new Bool(compare(this.v, other.v) >= 0);
	}

	le(other: Str): Bool {
		return new Bool(compare(this.v, other.v) <= 0);
	}

	to_owned(): String {
		return new String(this.v);
	}

	is_empty(): Bool {
		return new Bool(this.v.length === 0);
	}

	len(): F64 {
		return new F64(TEXT_ENCODER.encode(this.v).length);
	}

	trim(): Str {
		return new Str(this.v.replace(TRIM_START, "").replace(TRIM_END, ""));
	}

	trim_start(): Str {
		return new Str(this.v.replace(TRIM_START, ""));
	}

	trim_end(): Str {
		return new Str(this.v.replace(TRIM_END, ""));
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

	dehydrate(): string {
		return this.v;
	}

	toString(): string {
		return this.v.toString();
	}
}

// biome-ignore lint/suspicious/noShadowRestrictedNames: Surrogate type
export class String extends Str {
	// Mirrors Rust's `Deref<Target = str>`: dereferencing an owned string
	// yields the borrowed form. Returning a plain `Str` (never `this`) also
	// keeps ref-unwrapping loops finite (#192).
	deref(): Str {
		return new Str(this.v);
	}

	clone(): String {
		return new String(this.v);
	}

	dehydrate(): string {
		return this.v;
	}
}
