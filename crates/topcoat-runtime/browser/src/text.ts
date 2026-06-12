import { effect } from "@maverick-js/signals";

import type { Context } from "./context";
import type { Scope } from "./scope";
import { isNodeViewParts } from "./view";

type Compute = (ctx: Context) => unknown;

export function setupTextExpression(
	start: Comment,
	end: Comment,
	js: string,
	scope: Scope,
): void {
	const compute = new Function("cx", `return ${js};`) as Compute;
	const { context } = scope.runtime;

	let first = true;
	scope.run(() => {
		effect(() => {
			const value = compute(context);
			if (first) {
				first = false;
				return;
			}
			write(start, end, value);
		});
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
