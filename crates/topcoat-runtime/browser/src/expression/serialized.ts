import type { SignalId } from "../signal-registry";

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

export interface SerializedInteger {
	t: IntegerKind;
	bits: number;
	v: string;
}

export interface SerializedSequence {
	t: "Vec" | "Slice" | "Array";
	bits: number;
	v: DehydratedSurrogate[];
}

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
	| { t: "Procedure"; path: string };
