export class F64 {
	constructor(private readonly v: number) {}

	add(other: F64): F64 {
		return new F64(this.v + other.v);
	}

	sub(other: F64): F64 {
		return new F64(this.v - other.v);
	}

	mul(other: F64): F64 {
		return new F64(this.v * other.v);
	}

	div(other: F64): F64 {
		return new F64(this.v / other.v);
	}

	clone(): F64 {
		return new F64(this.v);
	}

	toJSON(): { t: "f64"; v: number } {
		return { t: "f64", v: this.v };
	}

	toString(): string {
		return this.v.toString();
	}

	valueOf(): number {
		return this.v;
	}
}
