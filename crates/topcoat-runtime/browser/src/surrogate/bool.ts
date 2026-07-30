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

	// `expr!` compiles `a && b` to `a.and(() => b)`. The right side arrives as
	// a thunk so it is only evaluated when the left side does not already
	// decide the result, which is what `&&` means on both sides.
	and(f: () => Bool): Bool {
		return this.v ? f() : this;
	}

	// The `||` mirror: the right side runs only when the left side is false.
	or(f: () => Bool): Bool {
		return this.v ? this : f();
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
