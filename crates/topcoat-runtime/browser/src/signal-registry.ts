import { signal, type WriteSignal } from "./reactivity";

/** A signal's id, as the server renders it: 32 hex digits. */
export type SignalId = string;

/** Every signal the document holds, keyed by id. */
export class SignalRegistry {
	private readonly signals = new Map<SignalId, WriteSignal<unknown>>();

	/** Whether a signal with this id exists. */
	has(id: SignalId): boolean {
		return this.signals.has(id);
	}

	/** Returns the signal with this id, if any. */
	get(id: SignalId): WriteSignal<unknown> | undefined {
		return this.signals.get(id);
	}

	/**
	 * Inserts a signal with the given id and initial value. If one already
	 * exists, the existing signal is kept and nothing changes. Returns whether
	 * a new signal was created.
	 */
	insert(id: SignalId, value: unknown): boolean {
		if (this.signals.has(id)) return false;
		this.signals.set(id, signal(value));
		return true;
	}

	/** Removes the signal with this id. */
	delete(id: SignalId): void {
		this.signals.delete(id);
	}

	/**
	 * Returns the signal handle for the given id. Calling the handle reads the
	 * current value and participates in dependency tracking. Throws if unknown.
	 */
	handle(id: SignalId): WriteSignal<unknown> {
		const s = this.signals.get(id);
		if (!s) throw new Error(`Unknown signal id: ${id}`);
		return s;
	}

	/**
	 * Reads the current value of a signal, participating in dependency tracking
	 * when called from inside an effect. Throws if the id is unknown.
	 */
	read(id: SignalId): unknown {
		return this.handle(id)();
	}
}
