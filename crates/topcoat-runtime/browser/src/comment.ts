import type { HandleId } from "./scope";

export type ReactiveScopeId = string;

export type CommentMarker =
	| { kind: "handle"; id: HandleId; value: unknown }
	| { kind: "expr-start"; js: string }
	| { kind: "expr-end" }
	| {
			kind: "scope-start";
			id: ReactiveScopeId;
			track: HandleId[];
			path: string;
	  }
	| { kind: "scope-end"; id: ReactiveScopeId };

const HANDLE_RE = /^::topcoat::handle\("([^"]+)", ([\s\S]*)\)$/;
const EXPR_START_RE = /^::topcoat::expr::start\("([^"]*)"\)$/;
const EXPR_END_RE = /^::topcoat::expr::end$/;
const SCOPE_START_RE =
	/^::topcoat::scope::start\(("[^"]+"), (\[[^\]]*\]), ("[^"]*")\)$/;
const SCOPE_END_RE = /^::topcoat::scope::end\(("[^"]+")\)$/;

export function parseComment(node: Comment): CommentMarker | null {
	const text = node.data.trim();

	const handle = HANDLE_RE.exec(text);
	if (handle) {
		return {
			kind: "handle",
			id: handle[1] as HandleId,
			value: JSON.parse(handle[2] ?? "") as unknown,
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
		return {
			kind: "scope-start",
			id: JSON.parse(start[1] ?? "") as ReactiveScopeId,
			track: JSON.parse(start[2] ?? "") as HandleId[],
			path: JSON.parse(start[3] ?? "") as string,
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
