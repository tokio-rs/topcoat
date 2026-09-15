import { afterEach, expect, it, vi } from "vitest";
import { SignalRegistry } from "../signal-registry";
import { Bool, F64, Option, Procedure, Result } from "../surrogate";
import { Context } from "./context";
import { dehydrate } from "./dehydrate";

afterEach(() => vi.unstubAllGlobals());

it("serializes nested options and results to the server's tagged format", () => {
	const value = Option.some(Result.from_ok(new F64(3)));
	const payload = JSON.parse(JSON.stringify(dehydrate(value)));
	expect(payload).toEqual({ t: "Option", v: { t: "Result", ok: 3 } });

	const cx = new Context(new SignalRegistry());
	const hydrated = cx.hydrate(payload) as Option<Result<F64, never>>;
	expect(hydrated.unwrap().unwrap().dehydrate()).toBe(3);
	expect(dehydrate(Result.from_err(Option.none()))).toEqual({
		t: "Result",
		err: { t: "Option", v: null },
	});
});

it("serializes a signal's nested value while keeping its identity", () => {
	const registry = new SignalRegistry();
	registry.insert("a", Option.some(new Bool(false)));
	const cx = new Context(registry);
	expect(dehydrate(cx.signal("a"))).toEqual({
		t: "Signal",
		id: "a",
		v: { t: "Option", v: false },
	});
});

it("sends recursively serialized procedure arguments when the future is awaited", async () => {
	const fetch = vi.fn(async () => new Response("null"));
	vi.stubGlobal("fetch", fetch);
	const procedure = new Procedure(new Context(new SignalRegistry()), "save");
	const pending = procedure.call(Option.some(new F64(4)));
	expect(fetch).not.toHaveBeenCalled();

	await pending;

	expect(fetch).toHaveBeenCalledWith(
		"/_topcoat/runtime/procedures/save",
		expect.objectContaining({
			body: JSON.stringify([{ t: "Option", v: 4 }]),
		}),
	);
});

it("rejects unsupported values instead of silently losing their state", () => {
	expect(() => dehydrate({ value: 3 })).toThrow(
		"Value cannot be serialized for the server",
	);
});
