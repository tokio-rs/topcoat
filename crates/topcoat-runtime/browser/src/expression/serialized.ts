import type { SignalId } from "../signal-registry";

export type DehydratedSurrogate =
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
