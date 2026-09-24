import {
	applyFrame,
	type FrameTarget,
	newRender,
	type RenderToken,
	type ServerMessage,
} from "./frames";

/** The WebSocket subprotocol used by the runtime. */
export const RUNTIME_PROTOCOL = "topcoat-runtime";

/** Receives rendered content and supplies the inputs for new renders. */
export interface ConnectionTarget extends FrameTarget {
	/**
	 * Collects the fields to send with a render request, such as the
	 * current signal values.
	 */
	renderInputs(): object;
	reportError(error: unknown): void;
}

/** Creates a WebSocket with the given URL and subprotocol. */
export type OpenSocket = (url: string, protocol: string) => WebSocket;

const defaultOpen: OpenSocket = (url, protocol) => new WebSocket(url, protocol);

/** The WebSocket state that allows sending messages. */
const OPEN = 1;

const INITIAL_RETRY_DELAY = 1000;
const MAX_RETRY_DELAY = 30_000;

/**
 * Requests server renders over a WebSocket at the target's URL.
 *
 * Each request starts a new run. The server sends the run id before its
 * output, allowing the browser to ignore updates from older runs.
 * If the connection closes, retries wait longer after each failed attempt.
 * Each successful connection starts a fresh render with the current inputs.
 */
export class Connection {
	private socket: WebSocket | null = null;
	private retry: ReturnType<typeof setTimeout> | null = null;
	/** Reconnect attempts since the last successful connection. */
	private attempt = 0;
	/** The most recent run id sent to the server. */
	private requested = 0;
	/** The run id announced by the server for incoming output. */
	private receiving = 0;
	/** The render the announced run's output belongs to. */
	private render: RenderToken = newRender();

	constructor(
		private readonly url: string,
		private readonly target: ConnectionTarget,
		private readonly lifetime: AbortSignal,
		private readonly open: OpenSocket = defaultOpen,
	) {
		lifetime.addEventListener("abort", () => this.close(), { once: true });
		this.connect();
	}

	/** Whether the socket is ready to send a render request. */
	get isOpen(): boolean {
		return this.socket?.readyState === OPEN;
	}

	/**
	 * Requests a new render with the target's current inputs.
	 * Does nothing while disconnected. Opening the connection requests a
	 * render automatically.
	 */
	requestRun(): void {
		if (this.socket === null || !this.isOpen) return;
		this.requested += 1;
		this.socket.send(
			JSON.stringify({ ...this.target.renderInputs(), run: this.requested }),
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
			// Do not reconnect if close() already cleared this socket.
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
				this.render = newRender();
				return;
			}
			// Ignore output if we have already requested a newer run.
			if (this.receiving !== this.requested) return;
			applyFrame(this.target, message, "Connected", this.render);
		} catch (error) {
			this.target.reportError(error);
		}
	}
}
