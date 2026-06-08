import type { AttributeValueViewParts, NodeViewParts } from "../view";
import { Bool } from "./bool";
import { Panic } from "./panic";

export class Option<T> implements AttributeValueViewParts, NodeViewParts {
	constructor(private readonly value: T | undefined) {}

	static some<T>(v: T): Option<T> {
		return new Option<T>(v);
	}

	static none<T>(): Option<T> {
		return new Option<T>(undefined);
	}

	is_some(): Bool {
		return new Bool(this.value !== undefined);
	}

	is_none(): Bool {
		return new Bool(this.value === undefined);
	}

	unwrap(): T {
		if (this.value === undefined) {
			throw new Panic("called `Option.unwrap()` on a `None` value");
		}
		return this.value;
	}

	expect(msg: { toString(): string }): T {
		if (this.value === undefined) {
			throw new Panic(msg.toString());
		}
		return this.value;
	}

	clone(): Option<T> {
		if (this.value === undefined) return Option.none<T>();
		const inner = this.value as { clone?: () => T };
		return Option.some<T>(
			typeof inner?.clone === "function" ? inner.clone() : this.value,
		);
	}

	isAttributePresent(): boolean {
		if (this.value === undefined) return false;
		return (this.value as AttributeValueViewParts).isAttributePresent();
	}

	toAttributeValue(): string {
		return (this.value as AttributeValueViewParts).toAttributeValue();
	}

	toNodeText(): string {
		if (this.value === undefined) return "";
		return (this.value as NodeViewParts).toNodeText();
	}

	toJSON(): { t: "Option"; v: unknown } {
		return { t: "Option", v: this.value === undefined ? null : this.value };
	}
}
