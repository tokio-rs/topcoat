import { afterEach, expect, it, vi } from "vitest";

import { Context } from "../expression/context";
import { SignalRegistry } from "../signal-registry";
import { Bool } from "./bool";
import { Procedure } from "./procedure";
import { String as RuntimeString } from "./string";

afterEach(() => vi.unstubAllGlobals());

it("sends argument arrays for empty, unit, and multiple arguments", async () => {
	const fetch = vi.fn(async () => new Response("true"));
	vi.stubGlobal("fetch", fetch);
	const procedure = new Procedure(
		new Context(new SignalRegistry()),
		"/api/example",
	);

	for (const [args, body] of [
		[[], "[]"],
		[[undefined], "[null]"],
		[[new Bool(true), new RuntimeString("hello")], '[true,"hello"]'],
	] as const) {
		await procedure.call(...args);
		expect(fetch).toHaveBeenLastCalledWith(
			"/api/example",
			expect.objectContaining({ method: "POST", body }),
		);
	}
});
