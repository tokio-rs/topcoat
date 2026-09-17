/** An opaque serialized Rust value. Its operations execute in Wasm. */
export class WasmValue {
	constructor(readonly value: unknown) {}

	dehydrate(): { t: "Wasm"; v: unknown } {
		return { t: "Wasm", v: this.value };
	}

	clone(): WasmValue {
		return new WasmValue(structuredClone(this.value));
	}

	toString(): string {
		return String(this.value);
	}

	valueOf(): unknown {
		return this.value;
	}
}
