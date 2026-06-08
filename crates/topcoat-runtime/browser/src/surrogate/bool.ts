export class Bool {
	constructor(private readonly v: boolean) {}

	clone(): Bool {
		return new Bool(this.v);
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
