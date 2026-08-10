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
			writeAttribute(el, name, compute(context));
		});
	});
}

export function writeAttribute(
	el: Element,
	name: string,
	value: unknown,
): void {
	if (PROPERTY_NAMES.has(name)) {
		(el as Element & Record<string, unknown>)[name] = value;
	}
	// A tuple, which hydrates as an array. `AttributeValueViewParts for
	// (T1, T2)` is present when any element is, and writes the elements one
	// after another; an element that is not present writes nothing, the way a
	// `None` does. Falling through would give `String(array)` and its commas.
	if (Array.isArray(value)) {
		const present = value.filter(
			(element) =>
				isAttributeValueViewParts(element) && element.isAttributePresent(),
		);
		if (present.length === 0) {
			el.removeAttribute(name);
			return;
		}
		el.setAttribute(name, present.map((e) => e.toAttributeValue()).join(""));
		return;
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
