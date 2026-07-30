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

/** The subset of those properties the DOM holds as a boolean. */
const BOOLEAN_PROPERTY_NAMES = new Set([
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

/**
 * Unwraps a computed value into the primitive its DOM property holds.
 *
 * A surrogate is an object and every object is truthy, so handing one straight
 * to a boolean property leaves the element checked (or selected) whatever the
 * expression evaluated to.
 */
export function propertyValue(name: string, value: unknown): unknown {
	const isBoolean = BOOLEAN_PROPERTY_NAMES.has(name);
	if (!isAttributeValueViewParts(value)) {
		return isBoolean ? Boolean(value) : value;
	}
	if (isBoolean) return value.isAttributePresent();
	// An absent value, such as a `None`, has no attribute value to read.
	return value.isAttributePresent() ? value.toAttributeValue() : "";
}

function write(el: Element, name: string, value: unknown): void {
	if (PROPERTY_NAMES.has(name)) {
		(el as Element & Record<string, unknown>)[name] = propertyValue(
			name,
			value,
		);
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
