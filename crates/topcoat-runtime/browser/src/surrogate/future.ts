/**
 * A Promise that only starts running after `.then` is called.
 * This mimics the behavior of Rust's Futures.
 */
export class Future<T> implements PromiseLike<T> {
	constructor(private v: () => Promise<T>) {}

	// biome-ignore lint/suspicious/noThenProperty: Future/Promise adapter
	then<TResult1 = T, TResult2 = never>(
		onfulfilled?:
			| ((value: T) => TResult1 | PromiseLike<TResult1>)
			| undefined
			| null,
		onrejected?:
			| ((reason: unknown) => TResult2 | PromiseLike<TResult2>)
			| undefined
			| null,
	): PromiseLike<TResult1 | TResult2> {
		const promise = this.v();
		this.v = () => promise;
		return promise.then(onfulfilled, onrejected);
	}
}
