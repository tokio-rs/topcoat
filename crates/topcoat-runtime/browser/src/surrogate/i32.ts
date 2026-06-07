export class I32 {
	constructor(private readonly v: number) {}

	add(other: I32): I32 {
		return new I32(this.v + other.v);
	}

	sub(other: I32): I32 {
		return new I32(this.v - other.v);
	}

	mul(other: I32): I32 {
		return new I32(this.v * other.v);
	}

	div(other: I32): I32 {
		return new I32(Math.trunc(this.v / other.v));
	}

	clone(): I32 {
		return new I32(this.v);
	}

	toJSON(): { t: "i32"; v: number } {
		return { t: "i32", v: this.v };
	}

	toString(): string {
		return this.v.toString();
	}

	valueOf(): number {
		return this.v;
	}
}
