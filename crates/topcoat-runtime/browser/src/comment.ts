import { Context } from "./context";
import { type SignalId, SignalRegistry } from "./signal";
import type { DehydratedSurrogate } from "./surrogate";

export type ShardScopeId = string;

export type CommentMarker =
	| { kind: "signal"; id: SignalId; value: unknown }
	| {
			/**
			 * The content depends on a signal: the server read it while
			 * rendering, so the innermost unit enclosing the marker re-runs
			 * when the signal changes.
			 */
			kind: "dep";
			id: SignalId;
	  }
	| { kind: "expr-start"; js: string }
	| { kind: "expr-end" }
	| {
			kind: "shard-start";
			id: ShardScopeId;
			/** The id of the shard, which names its route. */
			shard: string;
			/**
			 * The identity of the shard invocation, sent back with every
			 * re-render request so the server derives the same identities
			 * inside the shard as it did for the inline render.
			 */
			identity: string;
			exprs: string[];
	  }
	| { kind: "shard-end"; id: ShardScopeId };

const SIGNAL_RE = /^\s*::topcoat::signal\(([\s\S]*)\)\s*$/;
const DEP_RE = /^\s*::topcoat::dep\("([0-9a-f]+)"\)\s*$/;
const EXPR_START_RE = /^\s*::topcoat::expr::start\("([^"]*)"\)\s*$/;
const EXPR_END_RE = /^\s*::topcoat::expr::end\s*$/;
const SHARD_START_RE =
	/^\s*::topcoat::shard::start\(("[^"]+"), ("[^"]*"), ("[^"]*"), (\[[\s\S]*\])\)\s*$/;
const SHARD_END_RE = /^\s*::topcoat::shard::end\(("[^"]+")\)\s*$/;
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

	const dep = DEP_RE.exec(text);
	if (dep) {
		return { kind: "dep", id: dep[1] ?? "" };
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

	const start = SHARD_START_RE.exec(text);
	if (start) {
		const exprs: string[] = [];
		QUOTED_RE.lastIndex = 0;
		let m: RegExpExecArray | null = QUOTED_RE.exec(start[4] ?? "");
		while (m !== null) {
			exprs.push(decodeHtml(m[1] ?? ""));
			m = QUOTED_RE.exec(start[4] ?? "");
		}
		return {
			kind: "shard-start",
			id: JSON.parse(start[1] ?? "") as ShardScopeId,
			shard: JSON.parse(start[2] ?? "") as string,
			identity: JSON.parse(start[3] ?? "") as string,
			exprs,
		};
	}

	const end = SHARD_END_RE.exec(text);
	if (end) {
		return {
			kind: "shard-end",
			id: JSON.parse(end[1] ?? "") as ShardScopeId,
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
