import type { Context } from "./context";

export type Expression<T = unknown> = (cx: Context) => T;

/** Compiles server-generated source without evaluating it. */
export function compile<T = unknown>(
	source: string,
	label: string,
): Expression<T> {
	try {
		return new Function("cx", `return ${source};`) as Expression<T>;
	} catch (cause) {
		throw new Error(`Failed to compile ${label}`, { cause });
	}
}
