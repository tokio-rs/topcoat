import { effect } from "@maverick-js/signals";

import type { Context } from "./context";
import type { Scope } from "./scope";
import { isAttributeValueViewParts } from "./view";

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

type Compute = (ctx: Context) => unknown;

export function setupBinding(el: Element, attr: Attr, scope: Scope): void {
	if (!attr.name.startsWith(BIND_PREFIX)) return;

	const name = attr.name.substring(BIND_PREFIX.length);
	const compute = new Function("cx", `return ${attr.value};`) as Compute;

	const { context } = scope.runtime;
	scope.run(() => {
		effect(() => {
			write(el, name, compute(context));
		});
	});
}

function write(el: Element, name: string, value: unknown): void {
	if (PROPERTY_NAMES.has(name)) {
		(el as unknown as Record<string, unknown>)[name] = value;
	}
	if (isAttributeValueViewParts(value)) {
		if (!value.isAttributePresent()) {
			el.removeAttribute(name);
			return;
		}
		el.setAttribute(name, value.toAttributeValue());
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
