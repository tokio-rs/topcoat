import { effect } from "@maverick-js/signals";

import { type Expr, eval_expr } from "./expr";
import type { Scope } from "./scope";

export const BIND_PREFIX = "data-topcoat-bind:";

/**
 * Attribute names that must be set as DOM properties rather than HTML
 * attributes because the attribute represents the initial value, not the live
 * state.
 */
const PROPERTY_NAMES = new Set([
	"value",
	"checked",
	"selected",
	"indeterminate",
]);

export function setupBinding(el: Element, attr: Attr, scope: Scope): void {
	if (!attr.name.startsWith(BIND_PREFIX)) return;

	const name = attr.name.substring(BIND_PREFIX.length);
	const expr = JSON.parse(attr.value) as Expr;

	const { interpreter } = scope.runtime;
	scope.run(() => {
		effect(() => {
			write(el, name, eval_expr(expr, interpreter));
		});
	});
}

function write(el: Element, name: string, value: unknown): void {
	if (PROPERTY_NAMES.has(name)) {
		(el as unknown as Record<string, unknown>)[name] = value;
		return;
	}
	if (value == null || value === false) {
		el.removeAttribute(name);
		return;
	}
	if (value === true) {
		el.setAttribute(name, "");
		return;
	}
	el.setAttribute(name, String(value));
}
