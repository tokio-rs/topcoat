import { expect, it } from "vitest";
import { Bool } from "./bool";

it("preserves a Bool when it crosses a Promise boundary", async () => {
	// Procedure results resolve through a Promise. A callable `then` makes
	// JavaScript assimilate Bool as a thenable and previously yielded undefined.
	const result = await Promise.resolve(new Bool(true));

	expect(result).toBeInstanceOf(Bool);
	expect(result.dehydrate()).toBe(true);
});

it("retains Rust bool::then behavior without becoming a JavaScript thenable", () => {
	expect(new Bool(true).then_(() => "value").unwrap()).toBe("value");
	expect(new Bool(false).then_(() => "value").is_none().dehydrate()).toBe(true);
});
