/** An error that stands for a Rust panic. */
export class Panic extends Error {
	constructor(message: string) {
		super(message);
		this.name = "Panic";
	}
}
