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

	// `then` would make Bool a JavaScript thenable. Promise resolution would
	// invoke it and replace a procedure's boolean result with `undefined`.
	then_<T>(f: () => T): Option<T> {
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

	dehydrate(): boolean {
		return this.v;
	}

	toString(): string {
		return this.v.toString();
	}
}
