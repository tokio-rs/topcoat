import { describe, expect, it } from "vitest";
import { Context } from "../expression/context";
import { dehydrate } from "../expression/dehydrate";
import { Effect, flushEffects } from "../reactivity";
import { SignalRegistry } from "../signal-registry";
import type { WasmValue } from "./wasm";

describe("opaque Wasm values", () => {
	it("preserves nested data without interpreting record fields as surrogate tags", () => {
		const cx = new Context(new SignalRegistry());
		const data = { t: "Signal", id: "ordinary-data", lines: [{ price: 3.5 }] };
		const value = cx.hydrate({ t: "Wasm", v: data }) as WasmValue;
		expect(value.value).toEqual(data);
		expect(dehydrate(value)).toEqual({ t: "Wasm", v: data });
		const copy = value.clone();
		const line = (copy.value as typeof data).lines[0];
		if (!line) throw new Error("missing cloned line");
		line.price = 9;
		expect((value.value as typeof data).lines[0]?.price).toBe(3.5);
	});

	it("tracks whole-struct reads and notifies after replacement", () => {
		const registry = new SignalRegistry();
		const cx = new Context(registry);
		registry.insert("product", cx.hydrate({ t: "Wasm", v: { quantity: 1 } }));
		const values: number[] = [];
		const effect = new Effect(() => {
			const value = registry.read("product") as WasmValue;
			values.push((value.value as { quantity: number }).quantity);
		});
		effect.run();
		registry
			.handle("product")
			.set(cx.hydrate({ t: "Wasm", v: { quantity: 2 } }));
		flushEffects();
		expect(values).toEqual([1, 2]);
		effect.dispose();
	});
});
