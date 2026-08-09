import { describe, expect, it, vi } from "vitest";
import { Context } from "../context";
import { SignalRegistry } from "../signal";
import { Procedure } from "./procedure";
import { String as SurrogateString } from "./string";

describe("Procedure", () => {
	it("serialises native string arguments directly", async () => {
		const fetchMock = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => "ok",
		});
		vi.stubGlobal("fetch", fetchMock);

		const cx = new Context(new SignalRegistry());
		const procedure = new Procedure(cx, "test-id");
		await procedure.call("hello");
		expect(fetchMock).toHaveBeenCalledWith(
			"/_topcoat/procedures/test-id",
			expect.objectContaining({
				method: "POST",
				body: JSON.stringify(["hello"]),
			}),
		);

		vi.unstubAllGlobals();
	});

	it("dehydrates surrogate arguments", async () => {
		const fetchMock = vi.fn().mockResolvedValue({
			ok: true,
			json: async () => "ok",
		});
		vi.stubGlobal("fetch", fetchMock);

		const cx = new Context(new SignalRegistry());
		const procedure = new Procedure(cx, "test-id");
		await procedure.call(new SurrogateString("world"));

		expect(fetchMock).toHaveBeenCalledWith(
			"/_topcoat/procedures/test-id",
			expect.objectContaining({
				body: JSON.stringify(["world"]),
			}),
		);

		vi.unstubAllGlobals();
	});
});
