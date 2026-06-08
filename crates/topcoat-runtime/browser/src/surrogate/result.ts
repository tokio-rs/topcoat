import { Bool } from "./bool";
import { Option } from "./option";
import { Panic } from "./panic";

type ResultKind = "ok" | "err";

export class Result<T, E> {
	constructor(
		private readonly kind: ResultKind,
		private readonly value: T | E,
	) {}

	static from_ok<T, E = never>(v: T): Result<T, E> {
		return new Result<T, E>("ok", v);
	}

	static from_err<T = never, E = unknown>(v: E): Result<T, E> {
		return new Result<T, E>("err", v);
	}

	is_ok(): Bool {
		return new Bool(this.kind === "ok");
	}

	is_err(): Bool {
		return new Bool(this.kind === "err");
	}

	ok(): Option<T> {
		return this.kind === "ok" ? Option.some(this.value as T) : Option.none<T>();
	}

	err(): Option<E> {
		return this.kind === "err"
			? Option.some(this.value as E)
			: Option.none<E>();
	}

	unwrap(): T {
		if (this.kind === "err") {
			throw new Panic(
				`called \`Result.unwrap()\` on an \`Err\` value: ${formatPanicValue(this.value)}`,
			);
		}
		return this.value as T;
	}

	expect(msg: { toString(): string }): T {
		if (this.kind === "err") {
			throw new Panic(`${msg.toString()}: ${formatPanicValue(this.value)}`);
		}
		return this.value as T;
	}

	unwrap_err(): E {
		if (this.kind === "ok") {
			throw new Panic(
				`called \`Result.unwrap_err()\` on an \`Ok\` value: ${formatPanicValue(this.value)}`,
			);
		}
		return this.value as E;
	}

	expect_err(msg: { toString(): string }): E {
		if (this.kind === "ok") {
			throw new Panic(`${msg.toString()}: ${formatPanicValue(this.value)}`);
		}
		return this.value as E;
	}

	clone(): Result<T, E> {
		const inner = this.value as { clone?: () => T | E };
		const value =
			typeof inner?.clone === "function" ? inner.clone() : this.value;
		return this.kind === "ok"
			? Result.from_ok<T, E>(value as T)
			: Result.from_err<T, E>(value as E);
	}

	toJSON(): { t: "Result"; v: { ok: unknown } | { err: unknown } } {
		return {
			t: "Result",
			v: this.kind === "ok" ? { ok: this.value } : { err: this.value },
		};
	}
}

function formatPanicValue(value: unknown): string {
	if (typeof value === "string") return JSON.stringify(value);

	if (value !== null && value !== undefined) {
		const toStringMethod = (value as { toString?: unknown }).toString;
		if (
			typeof toStringMethod === "function" &&
			toStringMethod !== Object.prototype.toString
		) {
			return toStringMethod.call(value);
		}
	}

	const json = JSON.stringify(value);
	return json === undefined ? globalThis.String(value) : json;
}
