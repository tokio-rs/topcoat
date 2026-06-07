// biome-ignore lint/suspicious/noShadowRestrictedNames: Surrogate type
export class String {
	constructor(private readonly v: string) {}

	clone(): String {
		return new String(this.v);
	}

	toJSON(): { t: "String"; v: string } {
		return { t: "String", v: this.v };
	}

	toString(): string {
		return this.v.toString();
	}
}
