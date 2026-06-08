import type { AttributeValueViewParts, NodeViewParts } from "../view";
import { Option } from "./option";

export class Bool implements AttributeValueViewParts, NodeViewParts {
	constructor(private readonly v: boolean) {}

	clone(): Bool {
		return new Bool(this.v);
	}

	not(): Bool {
		return new Bool(!this.v);
	}

	eq(other: Bool): Bool {
		return new Bool(this.v === other.v);
	}

	ne(other: Bool): Bool {
		return new Bool(this.v !== other.v);
	}

	// biome-ignore lint/suspicious/noThenProperty: Intended behavior for cross compilation.
	then<T>(f: () => T): Option<T> {
		return this.v ? Option.some(f()) : Option.none<T>();
	}

	then_some<T>(t: T): Option<T> {
		return this.v ? Option.some(t) : Option.none<T>();
	}

	isAttributePresent(): boolean {
		return this.v;
	}

	toAttributeValue(): string {
		return "true";
	}

	toNodeText(): string {
		return this.v.toString();
	}

	toJSON(): { t: "bool"; v: boolean } {
		return { t: "bool", v: this.v };
	}

	toString(): string {
		return this.v.toString();
	}

	valueOf(): boolean {
		return this.v;
	}
}
