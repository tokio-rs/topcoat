import { dehydrate } from "../expression/dehydrate";
import type { SerializedSequence } from "../expression/serialized";
import { Bool } from "./bool";
import { Integer, type IntegerType } from "./integer";
import { Option } from "./option";
import { cloneValue, Ref } from "./ref";

/** A shared view of a sequence's elements. */
export class Slice<T> {
	constructor(
		protected readonly items: readonly T[],
		protected readonly usizeType: IntegerType,
		protected readonly start = 0,
		protected readonly end = items.length,
	) {
		if (
			usizeType.kind !== "usize" ||
			!Number.isInteger(start) ||
			!Number.isInteger(end) ||
			start < 0 ||
			end < start ||
			end > items.length
		) {
			throw new Error("Invalid slice");
		}
		this.len();
	}

	len(): Integer {
		return new Integer(BigInt(this.end - this.start), this.usizeType);
	}

	is_empty(): Bool {
		return new Bool(this.start === this.end);
	}

	get(index: Integer): Option<Ref<T> & T> {
		const offset = index.toIndex(this.end - this.start, this.usizeType.bits);
		return offset === undefined
			? Option.none()
			: Option.some(Ref.shared(() => this.items[this.start + offset] as T));
	}

	first(): Option<Ref<T> & T> {
		return this.get(new Integer(0n, this.usizeType));
	}

	index(index: Integer): Ref<T> & T {
		return this.get(index).unwrap();
	}

	last(): Option<Ref<T> & T> {
		return this.start === this.end
			? Option.none()
			: this.get(
					new Integer(BigInt(this.end - this.start - 1), this.usizeType),
				);
	}

	to_vec(): Vec<T> {
		return new Vec(
			this.items.slice(this.start, this.end).map(cloneValue),
			this.usizeType,
		);
	}

	to_owned(): Vec<T> {
		return this.to_vec();
	}
}

/** An owned sequence. Cloning duplicates its owned elements recursively. */
export class Vec<T> extends Slice<T> {
	constructor(items: readonly T[], usizeType: IntegerType) {
		super(items, usizeType);
	}

	as_slice(): Ref<Slice<T>> & Slice<T> {
		const slice = this.deref();
		return Ref.shared(() => slice);
	}

	deref(): Slice<T> {
		return new Slice(this.items, this.usizeType, this.start, this.end);
	}

	clone(): Vec<T> {
		return this.to_vec();
	}

	dehydrate(): SerializedSequence {
		return {
			t: "Vec",
			bits: this.usizeType.bits,
			v: this.items.map(dehydrate),
		};
	}
}

/** A fixed-size owned sequence with the same read methods as a slice. */
export class FixedArray<T> extends Slice<T> {
	constructor(items: readonly T[], usizeType: IntegerType) {
		super(items, usizeType);
	}

	as_slice(): Ref<Slice<T>> & Slice<T> {
		const slice = this.deref();
		return Ref.shared(() => slice);
	}

	deref(): Slice<T> {
		return new Slice(this.items, this.usizeType, this.start, this.end);
	}

	clone(): FixedArray<T> {
		return new FixedArray(this.items.map(cloneValue), this.usizeType);
	}

	to_owned(): FixedArray<T> {
		return this.clone();
	}

	dehydrate(): SerializedSequence {
		return {
			t: "Array",
			bits: this.usizeType.bits,
			v: this.items.map(dehydrate),
		};
	}
}
