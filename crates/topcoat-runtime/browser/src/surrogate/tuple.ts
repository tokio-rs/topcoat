import {
	type AttributeValueViewParts,
	isAttributeValueViewParts,
	isNodeViewParts,
	type NodeViewParts,
} from "../dom/view";
import { dehydrate } from "../expression/dehydrate";
import type { DehydratedSurrogate } from "../expression/serialized";
import { cloneValue } from "./ref";

/**
 * A fixed-size group of values. Elements are readable by index, so the
 * compiled field access `pair[0]` reads the first element.
 */
export class Tuple implements AttributeValueViewParts, NodeViewParts {
	[index: number]: unknown;

	constructor(private readonly items: readonly unknown[]) {
		Object.assign(this, items);
	}

	/** Returns whether `index` names an element of this tuple. */
	isIndex(index: number): boolean {
		return Number.isInteger(index) && index >= 0 && index < this.items.length;
	}

	[Symbol.iterator](): Iterator<unknown> {
		return this.items[Symbol.iterator]();
	}

	clone(): Tuple {
		return new Tuple(this.items.map(cloneValue));
	}

	isAttributePresent(): boolean {
		return this.items.some(
			(item) => isAttributeValueViewParts(item) && item.isAttributePresent(),
		);
	}

	toAttributeValue(): string {
		let value = "";
		for (const item of this.items) {
			if (isAttributeValueViewParts(item) && item.isAttributePresent()) {
				value += item.toAttributeValue();
			}
		}
		return value;
	}

	toNodeText(): string {
		let text = "";
		for (const item of this.items) {
			if (isNodeViewParts(item)) text += item.toNodeText();
		}
		return text;
	}

	dehydrate(): DehydratedSurrogate[] {
		return this.items.map(dehydrate);
	}

	toString(): string {
		const items = this.items.map((item) => globalThis.String(item));
		return items.length === 1 ? `(${items[0]},)` : `(${items.join(", ")})`;
	}
}
