import { expect, it } from "vitest";

import { propertyValue } from "./binding";
import { Bool } from "./surrogate/bool";
import { Option } from "./surrogate/option";
import { String } from "./surrogate/string";

// `value`, `checked`, `selected` and `indeterminate` are written as DOM
// properties, so the value a bind computes has to be unwrapped down to the
// primitive the property holds. A surrogate is an object, and an object is
// truthy, so `<input type="checkbox" :checked=$(open.get())>` used to render
// checked for a `false` signal too.
it("unwraps a bool surrogate for a boolean property", () => {
	expect(propertyValue("checked", new Bool(false))).toBe(false);
	expect(propertyValue("checked", new Bool(true))).toBe(true);
	expect(propertyValue("indeterminate", new Bool(false))).toBe(false);
});

it("reads absence as false for a boolean property", () => {
	expect(propertyValue("selected", Option.none())).toBe(false);
	expect(propertyValue("selected", Option.some(new Bool(true)))).toBe(true);
});

it("unwraps a string surrogate for the value property", () => {
	expect(propertyValue("value", new String("ada"))).toBe("ada");
	// `None` carries no attribute value, and an input holds "" for no value.
	expect(propertyValue("value", Option.none())).toBe("");
});

// Handlers can also compute plain JavaScript through `raw!`.
it("keeps plain values, coercing only where the property is a boolean", () => {
	expect(propertyValue("checked", true)).toBe(true);
	expect(propertyValue("checked", null)).toBe(false);
	expect(propertyValue("value", "ada")).toBe("ada");
});
