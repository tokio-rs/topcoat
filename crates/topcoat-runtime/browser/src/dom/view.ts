/**
 * Mirrors the Rust `AttributeValueViewParts` trait: a value that can be
 * rendered into an HTML attribute. `isAttributePresent` decides whether the
 * attribute should be set at all (boolean HTML attributes omit themselves
 * when false), and `toAttributeValue` produces the value string when it is.
 */
export interface AttributeValueViewParts {
	isAttributePresent(): boolean;
	toAttributeValue(): string;
}

export function isAttributeValueViewParts(
	value: unknown,
): value is AttributeValueViewParts {
	return (
		value !== null &&
		typeof value === "object" &&
		typeof (value as { isAttributePresent?: unknown }).isAttributePresent ===
			"function" &&
		typeof (value as { toAttributeValue?: unknown }).toAttributeValue ===
			"function"
	);
}

/**
 * Mirrors the Rust `NodeViewParts` trait: a value that can be rendered as
 * text in node position. `toNodeText` produces the text to insert; types
 * with no representation (e.g. `None`) return the empty string.
 */
export interface NodeViewParts {
	toNodeText(): string;
}

export function isNodeViewParts(value: unknown): value is NodeViewParts {
	return (
		value !== null &&
		typeof value === "object" &&
		typeof (value as { toNodeText?: unknown }).toNodeText === "function"
	);
}
