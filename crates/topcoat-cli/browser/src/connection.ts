export type DevEvent = "reload" | "rebuilding" | "build-failed" | "app-exited" | "up-to-date";

interface Handlers {
	event: (event: DevEvent) => void;
	reconnected: () => void;
	navigating: () => void;
}

/** Keeps a dev-server connection alive without interrupting navigation. */
export class DevConnection {
	private readonly url: string;
	private readonly lifetime = new AbortController();
	private socket: WebSocket | null = null;
	private retry: ReturnType<typeof setTimeout> | undefined;
	private navigating = false;

	constructor(scriptUrl: string, private readonly handlers: Handlers) {
		const url = new URL(scriptUrl);
		url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
		url.pathname = "/ws";
		url.search = "";
		url.hash = "";
		this.url = url.href;
	}

	get isNavigating(): boolean {
		return this.navigating;
	}

	start(): void {
		const options = { signal: this.lifetime.signal };
		const navigating = () => {
			this.navigating = true;
			this.handlers.navigating();
		};
		// Firefox closes the socket at navigation start, Chrome at commit.
		window.navigation?.addEventListener("navigate", navigating, options);
		window.addEventListener("pagehide", navigating, options);
		window.navigation?.addEventListener("navigateerror", () => { this.navigating = false; }, options);
		// Restoring a page from the back/forward cache resumes reconnects.
		window.addEventListener("pageshow", () => { this.navigating = false; }, options);
		this.connect(false);
	}

	stop(): void {
		this.lifetime.abort();
		clearTimeout(this.retry);
		const socket = this.socket;
		this.socket = null;
		socket?.close();
	}

	private connect(reconnecting: boolean): void {
		const socket = new WebSocket(this.url);
		this.socket = socket;
		socket.onopen = () => {
			if (this.socket === socket && reconnecting && !this.navigating) {
				this.handlers.reconnected();
			}
		};
		socket.onmessage = ({ data }: MessageEvent<unknown>) => {
			if (this.socket !== socket) return;
			switch (data) {
				case "reload":
				case "rebuilding":
				case "build-failed":
				case "app-exited":
				case "up-to-date":
					this.handlers.event(data);
			}
		};
		socket.onclose = () => {
			if (this.socket !== socket) return;
			this.socket = null;
			this.reconnect();
		};
	}

	private reconnect(): void {
		this.retry = setTimeout(() => {
			if (this.lifetime.signal.aborted) return;
			if (this.navigating) this.reconnect();
			else this.connect(true);
		}, 500);
	}
}
