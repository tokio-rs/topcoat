import type { Context } from "../../src/context";
import { SignalRegistry } from "../../src/signal";
import { Panic } from "../../src/surrogate/panic";
import { FixtureContext } from "./fixture";
import { observe } from "./observe";

/** Executes production-generated source in an independent runtime context. */
export function execute(source: string, invoke: boolean): string {
	const cx = new FixtureContext(new SignalRegistry());
	const compute = compile(source);
	if (typeof compute === "string") return compute;
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

/** Invokes an async closure and observes its completed value. */
export async function executeAsync(source: string): Promise<string> {
	const cx = new FixtureContext(new SignalRegistry());
	const compute = compile(source);
	if (typeof compute === "string") return compute;
	let value: unknown;
	try {
		const run: unknown = compute(cx);
		if (typeof run !== "function") {
			throw new Error("Expected a compiled async closure");
		}
		const pending: unknown = run();
		if (!(pending instanceof Promise)) {
			throw new Error("Compiled async closure must return a promise");
		}
		value = await pending;
	} catch (error) {
		if (error instanceof Panic) {
			return JSON.stringify({ kind: "Panic", value: error.message });
		}
		throw error;
	}
	return JSON.stringify({ kind: "Return", value: observe(value) });
}

function compile(source: string): ((cx: Context) => unknown) | string {
	try {
		return new Function("cx", `return (${source});`) as (
			cx: Context,
		) => unknown;
	} catch (error) {
		if (error instanceof SyntaxError) {
			return JSON.stringify({ kind: "CompileError", value: error.toString() });
		}
		throw error;
	}
}
