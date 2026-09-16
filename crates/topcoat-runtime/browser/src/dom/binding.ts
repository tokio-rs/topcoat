import { compile } from "../expression/compile";
import type { Scope } from "../scope";
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

export function setupBinding(el: Element, attr: Attr, scope: Scope): void {
	if (!attr.name.startsWith(BIND_PREFIX)) return;

	const name = attr.name.substring(BIND_PREFIX.length);
	const compute = compile(attr.value, `binding :${name}`);

	const { context } = scope.runtime;
	scope.effect(() => {
		write(el, name, compute(context));
	});
}

function write(el: Element, name: string, value: unknown): void {
	if (PROPERTY_NAMES.has(name)) {
		let propertyValue = value;
		if (isAttributeValueViewParts(value)) {
			const present = value.isAttributePresent();
			// DOM boolean properties need a primitive, not a truthy surrogate.
			propertyValue = name === "value"
				? (present ? value.toAttributeValue() : "")
				: present;
		}
		(el as Element & Record<string, unknown>)[name] = propertyValue;
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
