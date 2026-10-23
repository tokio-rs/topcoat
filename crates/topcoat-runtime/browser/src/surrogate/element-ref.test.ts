import { expect, it } from "vitest";
import { Context } from "../expression/context";
import { dehydrate } from "../expression/dehydrate";
import { SignalRegistry } from "../signal-registry";
import { ElementRef } from "./element-ref";

const context = new Context(new SignalRegistry());

it("hydrates and clones metadata without accessing a document", () => {
	const serialized = { t: "ElementRef", id: "field", kind: "input" };
	const reference = context.hydrate(serialized);
	expect(reference).toBeInstanceOf(ElementRef);
	if (!(reference instanceof ElementRef)) throw new Error("Missing reference");
	expect(dehydrate(reference.clone())).toEqual(serialized);
	expect(String(reference.id())).toBe(serialized.id);
});

it.each([
	{ id: null, kind: "input" },
	{ id: "field", kind: "unknown" },
	{ id: "field", kind: "toString" },
])("rejects invalid element metadata %j", (metadata) => {
	expect(() => context.hydrate({ t: "ElementRef", ...metadata })).toThrow();
});
