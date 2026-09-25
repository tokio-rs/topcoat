import type { DehydratedSurrogate } from "./serialized";

// Surrogates use this helper directly so encoding does not import their classes.
/** Converts a runtime value to the representation the server accepts. */
export function dehydrate(value: unknown): DehydratedSurrogate {
	if (value == null) return null;
	if (
		typeof value === "string" ||
		typeof value === "number" ||
		typeof value === "boolean"
	) {
		return value;
	}
	if (typeof value === "object") {
		const serializable = value as { dehydrate?: () => DehydratedSurrogate };
		if (typeof serializable.dehydrate === "function") {
			return serializable.dehydrate();
		}
	}
	throw new Error("Value cannot be serialized for the server");
}
