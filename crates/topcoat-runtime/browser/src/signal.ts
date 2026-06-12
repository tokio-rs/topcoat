import { signal, type WriteSignal } from "@maverick-js/signals";

export type SignalId = string;

export class SignalRegistry {
	private readonly signals = new Map<SignalId, WriteSignal<unknown>>();

	has(id: SignalId): boolean {
		return this.signals.has(id);
	}

	get(id: SignalId): WriteSignal<unknown> | undefined {
		return this.signals.get(id);
	}

	/**
	 * Inserts a signal with the given id. If one already exists, the call is a
	 * no-op (existing signal wins). Returns `true` iff a new signal was created.
	 */
	insert(id: SignalId, value: unknown): boolean {
		if (this.signals.has(id)) return false;
		this.signals.set(id, signal(value));
		return true;
	}

	delete(id: SignalId): void {
		this.signals.delete(id);
	}

	/**
	 * Returns the signal handle for the given id. Calling the handle reads the
	 * current value and participates in maverick tracking. Throws if unknown.
	 */
	handle(id: SignalId): WriteSignal<unknown> {
		const s = this.signals.get(id);
		if (!s) throw new Error(`Unknown signal id: ${id}`);
		return s;
	}

	/**
	 * Reads the current value of a signal, participating in maverick tracking
	 * when called from inside an effect. Throws if the id is unknown.
	 */
	read(id: SignalId): unknown {
		return this.handle(id)();
	}
}
