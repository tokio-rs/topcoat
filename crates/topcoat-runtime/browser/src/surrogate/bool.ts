import type { AttributeValueViewParts, NodeViewParts } from "../view";

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
