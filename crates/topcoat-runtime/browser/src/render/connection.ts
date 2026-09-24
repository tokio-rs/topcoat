import type { DehydratedSurrogate } from "../expression/serialized";
import type { SignalId } from "../signal-registry";

/** The WebSocket subprotocol a runtime connection is requested with. */
export const RUNTIME_PROTOCOL = "topcoat-runtime";

/** A message the server sends over a connection. */
type ServerMessage =
	| { t: "run"; id: number }
	| { t: "snapshot"; html: string }
	| { t: "swap"; region: string; html: string }
	| { t: "redirect"; location: string }
	| { t: "error"; status: number };

/** The content a connection renders. */
export interface ConnectionTarget {
	/** The signal values a run starts from. */
	collectSignals(): Record<SignalId, DehydratedSurrogate>;
	/** Applies a run's snapshot. */
	replaceContent(html: string): void;
	/** Applies a run's region swap. */
	applySwap(region: string, html: string): void;
	reportError(error: unknown): void;
}

/** Opens the WebSocket for a connection. */
export type OpenSocket = (url: string, protocol: string) => WebSocket;

const defaultOpen: OpenSocket = (url, protocol) => new WebSocket(url, protocol);

/** The `readyState` of a WebSocket that can send. */
const OPEN = 1;

const INITIAL_RETRY_DELAY = 1000;
const MAX_RETRY_DELAY = 30_000;

/**
 * A runtime connection: a WebSocket at the target's URL, over which the
 * server renders the target each time it is asked to.
 *
 * Each request starts a run. The server announces a run before its output,
 * so the output of a run that a later request superseded is dropped. A
 * connection that closes is reopened after a growing delay, and every
 * opening requests a fresh run, since the server keeps nothing between
 * connections.
 */
export class Connection {
	private socket: WebSocket | null = null;
	private retry: ReturnType<typeof setTimeout> | null = null;
	/** How many times in a row the connection had to be reopened. */
	private attempt = 0;
	/** The number of the last run requested. */
	private requested = 0;
	/** The number of the run whose output the server is sending. */
	private receiving = 0;

	constructor(
		private readonly url: string,
		private readonly target: ConnectionTarget,
		private readonly lifetime: AbortSignal,
		private readonly open: OpenSocket = defaultOpen,
	) {
		lifetime.addEventListener("abort", () => this.close(), { once: true });
		this.connect();
	}

	/** Whether a run can be requested right now. */
	get isOpen(): boolean {
		return this.socket?.readyState === OPEN;
	}

	/**
	 * Asks the server to render the target again with its current signal
	 * values. Does nothing while the connection is not open: opening it
	 * requests a run anyway.
	 */
	requestRun(): void {
		if (this.socket === null || !this.isOpen) return;
		this.requested += 1;
		this.socket.send(
			JSON.stringify({
				run: this.requested,
				signals: this.target.collectSignals(),
			}),
		);
	}

	private connect(): void {
		if (this.lifetime.aborted) return;
		const socket = this.open(this.url, RUNTIME_PROTOCOL);
		this.socket = socket;
		socket.addEventListener("open", () => {
			this.attempt = 0;
			this.requestRun();
		});
		socket.addEventListener("message", (event) => this.receive(event.data));
		socket.addEventListener("close", () => {
			// A socket this connection closed itself is not reopened.
			if (this.socket !== socket) return;
			this.socket = null;
			this.scheduleReconnect();
		});
	}

	private scheduleReconnect(): void {
		if (this.lifetime.aborted || this.retry !== null) return;
		const delay = Math.min(
			INITIAL_RETRY_DELAY * 2 ** this.attempt,
			MAX_RETRY_DELAY,
		);
		this.attempt += 1;
		this.retry = setTimeout(() => {
			this.retry = null;
			this.connect();
		}, delay);
	}

	private close(): void {
		if (this.retry !== null) {
			clearTimeout(this.retry);
			this.retry = null;
		}
		const socket = this.socket;
		this.socket = null;
		socket?.close();
	}

	private receive(data: unknown): void {
		if (typeof data !== "string") return;
		try {
			const message = JSON.parse(data) as ServerMessage;
			if (message.t === "run") {
				this.receiving = message.id;
				return;
			}
			// Output of a run that a later request superseded.
			if (this.receiving !== this.requested) return;
			this.apply(message);
		} catch (error) {
			this.target.reportError(error);
		}
	}

	private apply(message: Exclude<ServerMessage, { t: "run" }>): void {
		switch (message.t) {
			case "snapshot":
				this.target.replaceContent(message.html);
				break;
			case "swap":
				this.target.applySwap(message.region, message.html);
				break;
			case "redirect":
				location.assign(message.location);
				break;
			case "error":
				throw new Error(`Connected render failed: ${message.status}`);
		}
	}
}
