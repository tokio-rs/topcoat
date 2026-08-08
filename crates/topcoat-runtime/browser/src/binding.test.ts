import { expect, it } from "vitest";

import { writeAttribute } from "./binding";
import { Bool } from "./surrogate/bool";
import { Option } from "./surrogate/option";
import { String as Owned } from "./surrogate/string";

/** The two calls `writeAttribute` makes, without needing a real DOM. */
function element() {
	const attrs = new Map<string, string>();
	return {
		attrs,
		el: {
			setAttribute: (n: string, v: string) => void attrs.set(n, v),
			removeAttribute: (n: string) => void attrs.delete(n),
		} as unknown as Element,
	};
}

// `AttributeValueViewParts for (T1, T2)` concatenates its elements and is
// present when any element is, so these are the attributes the server wrote.

it("writes a tuple attribute by concatenating its elements", () => {
	const { attrs, el } = element();

	writeAttribute(el, "title", [new Owned("a"), new Owned("b")]);

	expect(attrs.get("title")).toBe("ab");
});

it("skips an absent element inside a tuple attribute", () => {
	const { attrs, el } = element();

	writeAttribute(el, "title", [new Owned("a"), Option.none(), new Owned("b")]);

	expect(attrs.get("title")).toBe("ab");
});

it("removes the attribute when no element of a tuple is present", () => {
	const { attrs, el } = element();
	attrs.set("title", "stale");

	// `false` is a bool that is not present, the way a bare `false` attribute is.
	writeAttribute(el, "title", [Option.none(), new Bool(false)]);

	expect(attrs.has("title")).toBe(false);
});

it("still writes a plain surrogate attribute", () => {
	const { attrs, el } = element();

	writeAttribute(el, "title", new Owned("a"));

	expect(attrs.get("title")).toBe("a");
});
