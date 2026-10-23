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

export interface SerializedElementRef {
	t: "ElementRef";
	id: string;
	kind: "html" | "input" | "dialog";
}

export type DehydratedSurrogate =
	| SerializedElementRef
	| SerializedInteger
	| SerializedSequence
	| DehydratedSurrogate[]
	| null
	| boolean
	| number
	| { t: "str"; v: string }
	| string
	| { t: "Option"; v: DehydratedSurrogate | null }
	| { t: "Result"; ok: DehydratedSurrogate }
	| { t: "Result"; err: DehydratedSurrogate }
	| { t: "Record"; v: { [field: string]: DehydratedSurrogate } }
	| { t: "Signal"; id: SignalId; v?: DehydratedSurrogate }
	| { t: "Procedure"; path: string };
