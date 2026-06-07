export class Str {
	constructor(private readonly v: string) {}

	toJSON(): { t: "str"; v: string } {
		return { t: "str", v: this.v };
	}

	toString(): string {
		return this.v.toString();
	}
}
