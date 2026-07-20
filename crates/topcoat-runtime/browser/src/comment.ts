import { Context } from "./context";
import { type SignalId, SignalRegistry } from "./signal";
import type { DehydratedSurrogate } from "./surrogate";

export type ReactiveScopeId = string;

export type CommentMarker =
	| { kind: "signal"; id: SignalId; value: unknown }
	| { kind: "expr-start"; js: string }
	| { kind: "expr-end" }
	| {
			kind: "scope-start";
			id: ReactiveScopeId;
			path: string;
			exprs: string[];
	  }
	| { kind: "scope-end"; id: ReactiveScopeId };

const SIGNAL_RE = /^\s*::topcoat::signal\(([\s\S]*)\)\s*$/;
const EXPR_START_RE = /^\s*::topcoat::expr::start\("([^"]*)"\)\s*$/;
const EXPR_END_RE = /^\s*::topcoat::expr::end\s*$/;
const SCOPE_START_RE =
	/^\s*::topcoat::scope::start\(("[^"]+"), ("[^"]*"), (\[[\s\S]*\])\)\s*$/;
const SCOPE_END_RE = /^\s*::topcoat::scope::end\(("[^"]+")\)\s*$/;
const QUOTED_RE = /"([^"]*)"/g;

export function parseComment(node: Comment): CommentMarker | null {
	const text = node.data;

	const sig = SIGNAL_RE.exec(text);
	if (sig) {
		type SignalPayload = {
			t: "signal";
			id: SignalId;
			v: DehydratedSurrogate;
		};

		const payload = JSON.parse(decodeHtml(sig[1] ?? "")) as SignalPayload;
		if (payload.t !== "signal" || typeof payload.id !== "string") {
			throw new Error("Invalid signal marker");
		}
		const value = new Context(new SignalRegistry()).hydrate(payload.v);
		return {
			kind: "signal",
			id: payload.id,
			value,
		};
	}

	const exprStart = EXPR_START_RE.exec(text);
	if (exprStart) {
		const js = decodeHtml(exprStart[1] ?? "");
		return {
			kind: "expr-start",
			js,
		};
	}

	if (EXPR_END_RE.test(text)) {
		return { kind: "expr-end" };
	}

	const start = SCOPE_START_RE.exec(text);
	if (start) {
		const exprs: string[] = [];
		QUOTED_RE.lastIndex = 0;
		let m: RegExpExecArray | null = QUOTED_RE.exec(start[3] ?? "");
		while (m !== null) {
			exprs.push(decodeHtml(m[1] ?? ""));
			m = QUOTED_RE.exec(start[3] ?? "");
		}
		return {
			kind: "scope-start",
			id: JSON.parse(start[1] ?? "") as ReactiveScopeId,
			path: JSON.parse(start[2] ?? "") as string,
			exprs,
		};
	}

	const end = SCOPE_END_RE.exec(text);
	if (end) {
		return {
			kind: "scope-end",
			id: JSON.parse(end[1] ?? "") as ReactiveScopeId,
		};
	}

	return null;
}

function decodeHtml(value: string): string {
	const decoded = new DOMParser().parseFromString(value, "text/html")
		.documentElement.textContent;
	if (decoded === null) throw new Error("Failed to decode comment marker");
	return decoded;
}
