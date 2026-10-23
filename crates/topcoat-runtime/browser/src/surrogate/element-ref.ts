import type { SerializedElementRef } from "../expression/serialized";
import { Bool } from "./bool";
import { Result } from "./result";
import { Str } from "./string";

/** Identity and type metadata only. Never retains a DOM node. */
export class ElementRef {
	constructor(
		private readonly elementId: string,
		private readonly kind: SerializedElementRef["kind"],
	) {
		if (
			typeof elementId !== "string" ||
			!["html", "input", "dialog"].includes(kind)
		) {
			throw new Error("Invalid element reference");
		}
	}

	id(): Str {
		return new Str(this.elementId);
	}

	resolve(): Result<ElementHandle, ElementRefError> {
		const element = document.getElementById(this.elementId);
		if (element === null) {
			return Result.from_err(new ElementRefError("missing"));
		}
		const constructors = {
			html: HTMLElement,
			input: HTMLInputElement,
			dialog: HTMLDialogElement,
		};
		if (!(element instanceof constructors[this.kind])) {
			return Result.from_err(new ElementRefError("wrong-type"));
		}
		return Result.from_ok(new ElementHandle(element));
	}

	clone(): ElementRef {
		return new ElementRef(this.elementId, this.kind);
	}

	dehydrate(): SerializedElementRef {
		return { t: "ElementRef", id: this.elementId, kind: this.kind };
	}
}

/** A resolved node for browser operations. Deliberately not serializable. */
export class ElementHandle {
	constructor(readonly node: HTMLElement) {}

	clone(): ElementHandle {
		return new ElementHandle(this.node);
	}
}

export class ElementRefError {
	constructor(private readonly reason: "missing" | "wrong-type") {}

	is_missing(): Bool {
		return new Bool(this.reason === "missing");
	}

	is_wrong_type(): Bool {
		return new Bool(this.reason === "wrong-type");
	}
}
