import { compile } from "../expression/compile";
import type { Scope } from "../scope";
import { isNodeViewParts } from "./view";

/**
 * Keeps the text between an expression's start and end markers in sync with
 * the expression. The first run only subscribes, since the server already
 * rendered the initial text.
 */
export function setupTextExpression(
	start: Comment,
	end: Comment,
	js: string,
	scope: Scope,
): void {
	const compute = compile(js, "text expression");
	const { context } = scope.runtime;

	let first = true;
	scope.effect(() => {
		const value = compute(context);
		if (first) {
			first = false;
			return;
		}
		write(start, end, value);
	});
}

function write(start: Comment, end: Comment, value: unknown): void {
	const parent = start.parentNode;
	if (!parent) return;

	let n: ChildNode | null = start.nextSibling;
	while (n && n !== end) {
		const next: ChildNode | null = n.nextSibling;
		parent.removeChild(n);
		n = next;
	}

	const text = toText(value);
	if (text.length > 0) {
		parent.insertBefore(document.createTextNode(text), end);
	}
}

function toText(value: unknown): string {
	let current = value;
	while (isRefLike(current)) {
		current = current.deref();
	}
	if (current == null) return "";
	if (isNodeViewParts(current)) return current.toNodeText();
	return String(current);
}

function isRefLike(value: unknown): value is { deref: () => unknown } {
	return (
		value !== null &&
		typeof value === "object" &&
		typeof (value as { deref?: unknown }).deref === "function"
	);
}
