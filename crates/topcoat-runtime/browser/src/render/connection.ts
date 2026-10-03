import {
	applyFrame,
	type FrameTarget,
	newRender,
	type RenderToken,
	type ServerMessage,
} from "./frames";

/** The WebSocket subprotocol used by the runtime. */
export const RUNTIME_PROTOCOL = "topcoat-runtime";

/**
 * The `POST` request that re-renders content with its current inputs. Sent
 * over HTTP, or over the connection for a connected render.
 */
export type RerunRequest = {
	url: string;
	/** Only runtime headers and `Content-Type`; the connection rejects others. */
	headers: Record<string, string>;
	body: string;
};

/** Receives rendered content and describes the request for new renders. */
export interface ConnectionTarget extends FrameTarget {
	rerunRequest(): RerunRequest;
	reportError(error: unknown): void;
	/**
	 * Called for each member once the connection opens or reopens. The
	 * server keeps nothing between connections, so a member that still
	 * needs a connected render requests a run.
	 */
	connectionOpened(): void;
}

/** Creates a WebSocket with the given URL and subprotocol. */
export type OpenSocket = (url: string, protocol: string) => WebSocket;

const defaultOpen: OpenSocket = (url, protocol) => new WebSocket(url, protocol);

/** A message from the server: a frame of one run's output. */
type ConnectionMessage = {
	/** The run the frame belongs to. Absent for a rejected request. */
	run?: number;
	frame: ServerMessage;
};

/** The output of one run and the render it belongs to. */
type Run = { target: ConnectionTarget; render: RenderToken };

/** The WebSocket state that allows sending messages. */
const OPEN = 1;

const INITIAL_RETRY_DELAY = 1000;
const MAX_RETRY_DELAY = 30_000;

/**
 * One WebSocket for the whole document, carrying the connected renders of
 * any number of targets side by side.
 *
 * Targets that need a connected render join as members. The socket opens
 * when the first member joins and closes once none remain. Any target can
 * request a run while the socket is open, and each target has at most one
 * run at a time: a new run stops its previous one. Every frame names its
 * run, so output from a stopped run is dropped.
 *
 * If the connection closes while members remain, retries wait longer
 * after each failed attempt. Each successful connection lets every member
 * start a fresh run.
 */
export class Connection {
	private socket: WebSocket | null = null;
	private retry: ReturnType<typeof setTimeout> | null = null;
	/** Reconnect attempts since the last successful connection. */
	private attempt = 0;
	/** The most recent run id sent to the server. */
	private lastRun = 0;
	private readonly members = new Set<ConnectionTarget>();
	/** The current run of each target that has one. */
	private readonly runIds = new Map<ConnectionTarget, number>();
	/** The runs whose output is still accepted, by id. */
	private readonly runs = new Map<number, Run>();

	/**
	 * `url` returns the HTTP URL the socket opens at. `reportError`
	 * receives failures that belong to no run.
	 */
	constructor(
		private readonly url: () => string,
		private readonly reportError: (error: unknown) => void,
		private readonly open: OpenSocket = defaultOpen,
	) {}

	/** Whether the socket is ready to send a render request. */
	get isOpen(): boolean {
		return this.socket?.readyState === OPEN;
	}

	/**
	 * Makes `target` a member, opening the socket if needed. A member of
	 * an open connection without a current run runs right away.
	 */
	join(target: ConnectionTarget): void {
		if (this.members.has(target)) {
			// A member whose run was stopped, such as a page after
			// navigation, needs a new one.
			if (this.isOpen && !this.runIds.has(target)) this.run(target);
			return;
		}
		this.members.add(target);
		if (this.isOpen) {
			this.run(target);
		} else if (this.socket === null && this.retry === null) {
			this.connect();
		}
	}

	/**
	 * Ends the membership of `target` but keeps its current run. The socket
	 * closes once no members remain, unless one joins before the current
	 * task ends.
	 */
	leave(target: ConnectionTarget): void {
		if (!this.members.delete(target) || this.members.size > 0) return;
		queueMicrotask(() => {
			if (this.members.size === 0) this.disconnect();
		});
	}

	/**
	 * Requests a new run of `target` with its current inputs, stopping its
	 * previous run. Returns `false` without sending anything while the
	 * socket is not open.
	 */
	run(target: ConnectionTarget): boolean {
		const socket = this.socket;
		if (socket === null || !this.isOpen) return false;
		this.stop(target);
		this.lastRun += 1;
		const run = this.lastRun;
		this.runIds.set(target, run);
		this.runs.set(run, { target, render: newRender() });
		const { url, headers, body } = target.rerunRequest();
		const { pathname, search } = new URL(url, location.href);
		const path = `${pathname}${search}`;
		socket.send(JSON.stringify({ run, method: "POST", path, headers, body }));
		return true;
	}

	/** Stops the current run of `target`, if it has one. */
	stop(target: ConnectionTarget): void {
		const run = this.runIds.get(target);
		if (run === undefined) return;
		this.runIds.delete(target);
		this.runs.delete(run);
		if (this.isOpen) this.socket?.send(JSON.stringify({ stop: run }));
	}

	/** Stops the run of `target` and ends its membership. */
	remove(target: ConnectionTarget): void {
		this.stop(target);
		this.leave(target);
	}

	private connect(): void {
		const url = new URL(this.url(), location.href);
		url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
		const socket = this.open(url.href, RUNTIME_PROTOCOL);
		this.socket = socket;
		socket.addEventListener("open", () => {
			if (this.socket !== socket) return;
			this.attempt = 0;
			for (const member of [...this.members]) member.connectionOpened();
		});
		socket.addEventListener("message", (event) => this.receive(event.data));
		socket.addEventListener("close", () => {
			// Do not reconnect if disconnect() already cleared this socket.
			if (this.socket !== socket) return;
			this.socket = null;
			this.forgetRuns();
			this.scheduleReconnect();
		});
	}

	private scheduleReconnect(): void {
		if (this.members.size === 0 || this.retry !== null) return;
		const delay = Math.min(
			INITIAL_RETRY_DELAY * 2 ** this.attempt,
			MAX_RETRY_DELAY,
		);
		this.attempt += 1;
		this.retry = setTimeout(() => {
			this.retry = null;
			if (this.members.size > 0) this.connect();
		}, delay);
	}

	private disconnect(): void {
		if (this.retry !== null) {
			clearTimeout(this.retry);
			this.retry = null;
		}
		const socket = this.socket;
		this.socket = null;
		this.forgetRuns();
		this.attempt = 0;
		socket?.close();
	}

	/** The server drops every run with its connection. */
	private forgetRuns(): void {
		this.runIds.clear();
		this.runs.clear();
	}

	private receive(data: unknown): void {
		if (typeof data !== "string") return;
		let message: ConnectionMessage;
		try {
			message = JSON.parse(data) as ConnectionMessage;
		} catch (error) {
			this.reportError(error);
			return;
		}
		if (message.run === undefined) {
			const status = message.frame.t === "error" ? message.frame.status : "";
			this.reportError(new Error(`Connected request rejected: ${status}`));
			return;
		}
		// Output of a stopped or superseded run is dropped.
		const run = this.runs.get(message.run);
		if (run === undefined) return;
		try {
			applyFrame(run.target, message.frame, "Connected", run.render);
		} catch (error) {
			run.target.reportError(error);
		}
	}
}
