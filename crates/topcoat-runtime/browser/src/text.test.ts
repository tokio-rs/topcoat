import { expect, it } from "vitest";

import { F64 } from "./surrogate/f64";
import { Option } from "./surrogate/option";
import { String as Owned } from "./surrogate/string";
import { toText } from "./text";

// `NodeViewParts for (T1, T2)` writes its elements one after another with no
// separator, so these are the strings the server rendered for the same values.

it("renders a tuple by concatenating its elements", () => {
	expect(toText([new F64(1.5), new F64(2.5)])).toBe("1.52.5");
	expect(toText([new Owned("a"), new Owned("b")])).toBe("ab");
});

it("renders an absent element in a tuple as nothing", () => {
	expect(toText([new Owned("a"), Option.none(), new Owned("b")])).toBe("ab");
});

it("renders a nested tuple flat", () => {
	expect(toText([[new Owned("a"), new Owned("b")], new Owned("c")])).toBe(
		"abc",
	);
});

it("renders an empty tuple as nothing", () => {
	expect(toText([])).toBe("");
});
