import { Context } from "../../src/context";
import { SignalRegistry } from "../../src/signal";
import { Panic } from "../../src/surrogate/panic";
import { observe } from "./observe";

/** Executes production-generated source in an independent runtime context. */
export function execute(source: string, invoke: boolean): string {
	const cx = new Context(new SignalRegistry());
	const compute = new Function("cx", `return (${source});`);
	let value: unknown;
	try {
		value = compute(cx);
		if (invoke) {
			if (typeof value !== "function") {
				throw new Error("Expected a compiled closure");
			}
			value = value();
		}
	} catch (error) {
		if (error instanceof Panic) {
			return JSON.stringify({ kind: "Panic", value: error.message });
		}
		throw error;
	}
	return JSON.stringify({ kind: "Return", value: observe(value) });
}
