import type { SignalId } from "../signal-registry";

/** The name of a Rust integer type. */
export type IntegerKind =
	| "u8"
	| "u16"
	| "u32"
	| "u64"
	| "u128"
	| "usize"
	| "i8"
	| "i16"
	| "i32"
	| "i64"
	| "i128"
	| "isize";

/** A Rust integer as the server serializes it: type, width, and decimal digits. */
export interface SerializedInteger {
	t: IntegerKind;
	bits: number;
	v: string;
}

/** A vector, array, or slice as the server serializes it. */
export interface SerializedSequence {
	t: "Vec" | "Slice" | "Array";
	bits: number;
	v: DehydratedSurrogate[];
}

/** Any value in the form the server serializes it and accepts it back. */
export type DehydratedSurrogate =
	| SerializedInteger
	| SerializedSequence
	| null
	| boolean
	| number
	| { t: "str"; v: string }
	| string
	| { t: "Option"; v: DehydratedSurrogate | null }
	| { t: "Result"; ok: DehydratedSurrogate }
	| { t: "Result"; err: DehydratedSurrogate }
	| { t: "Signal"; id: SignalId; v?: DehydratedSurrogate }
	| { t: "Procedure"; id: string };
