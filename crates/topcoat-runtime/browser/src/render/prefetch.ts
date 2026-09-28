import { FRAMES_MEDIA_TYPE, readFrames, type ServerMessage } from "./frames";
import { RUNTIME_HEADER } from "./request";

/** How long to keep a prefetched page, in milliseconds. */
const PREFETCH_TTL = 30_000;
/** Cache size limit. New entries replace the oldest when the cache is full. */
const MAX_PREFETCHES = 8;
/** Limit on simultaneous prefetch requests. Skip new requests at this limit. */
const MAX_LOADING = 2;
/** Limit on buffered HTML text. Cancel prefetches that exceed it. */
const MAX_BUFFERED = 1024 * 1024;

/** Event name used to tell caches to discard their prefetched pages. */
export const INVALIDATE_PREFETCHES_EVENT = "topcoat:invalidate-prefetches";

/** Tells caches to clear pages that may be out of date after a server change. */
export function invalidatePrefetches(): void {
	if (typeof window === "undefined") return;
	window.dispatchEvent(new Event(INVALIDATE_PREFETCHES_EVENT));
}

/** Details about a page response, available before its body finishes loading. */
export type PageResponseHead = {
	/** The final response URL after following HTTP redirects. */
	url: string;
	/** Whether the response uses the runtime's render frame format. */
	frames: boolean;
	/** Expiration time for reusing this response, measured by `performance.now()`. */
	usableUntil: number;
};

/**
 * Requests a server render with the supplied signal values and stores
 * response frames until a reader consumes them.
 *
 * A reader gets every frame in order, starting with any already received.
 * This lets navigation reuse a prefetch response and keep reading it as
 * more frames arrive.
 */
export class PageRequest {
	readonly head: Promise<PageResponseHead>;
	/** Response details, or `null` while waiting for the headers. */
	private arrived: PageResponseHead | null = null;
	private readonly controller = new AbortController();
	private readonly buffer: ServerMessage[] = [];
	private buffered = 0;
	/** Buffer size limit before reading starts. `null` means no limit. */
	private limit: number | null;
	private finished = false;
	private failure: unknown = null;
	private wake: (() => void) | null = null;
	/** Resolves when the response finishes, fails, or is canceled. */
	readonly done: Promise<void>;
	private resolveDone!: () => void;

	constructor(
		/** The page URL with the # fragment removed. */
		readonly url: string,
		/** Signal values encoded as a JSON request body. */
		readonly body: string,
		limit: number | null = null,
	) {
		this.limit = limit;
		this.done = new Promise((resolve) => {
			this.resolveDone = resolve;
		});
		this.head = this.start();
		// Handle rejection even if the caller never awaits the response details.
		this.head.catch(() => undefined);
	}

	get isFinished(): boolean {
		return this.finished;
	}

	get failed(): boolean {
		return this.failure !== null;
	}

	/**
	 * Returns false if the request failed or the response expired.
	 * A request still waiting for headers can be reused.
	 */
	get usable(): boolean {
		if (this.failed) return false;
		return (
			this.arrived === null || performance.now() <= this.arrived.usableUntil
		);
	}

	abort(): void {
		this.controller.abort();
		this.fail(new DOMException("Page request aborted", "AbortError"));
	}

	/** Reads buffered frames, then waits for more until the response ends. */
	async *frames(): AsyncGenerator<ServerMessage> {
		this.limit = null;
		for (;;) {
			const frame = this.buffer.shift();
			if (frame !== undefined) {
				yield frame;
				continue;
			}
			if (this.failure !== null) throw this.failure;
			if (this.finished) return;
			await new Promise<void>((resolve) => {
				this.wake = resolve;
			});
		}
	}

	private async start(): Promise<PageResponseHead> {
		const response = await fetch(this.url, {
			method: "POST",
			cache: "no-store",
			headers: {
				"Content-Type": "application/json",
				[RUNTIME_HEADER]: "true",
				Accept: FRAMES_MEDIA_TYPE,
			},
			body: this.body,
			signal: this.controller.signal,
		}).catch((error: unknown) => {
			this.fail(error);
			throw error;
		});
		const mediaType = response.headers
			.get("Content-Type")
			?.split(";")[0]
			?.trim()
			.toLowerCase();
		const frames = mediaType === FRAMES_MEDIA_TYPE;
		const head = {
			url: response.url || this.url,
			frames,
			usableUntil: performance.now() + usableFor(response.headers),
		};
		this.arrived = head;
		if (frames) {
			void this.pump(response);
		} else {
			// Let a normal browser page load handle other response formats.
			this.controller.abort();
			this.finish();
		}
		return head;
	}

	private async pump(response: Response): Promise<void> {
		try {
			for await (const frame of readFrames(response)) {
				if (this.failure !== null) return;
				this.buffer.push(frame);
				this.buffered += "html" in frame ? frame.html.length : 0;
				if (this.limit !== null && this.buffered > this.limit) {
					this.abort();
					return;
				}
				this.notify();
			}
			this.finish();
		} catch (error) {
			this.fail(error);
		}
	}

	private finish(): void {
		this.finished = true;
		this.notify();
		this.resolveDone();
	}

	private fail(error: unknown): void {
		if (this.finished || this.failure !== null) return;
		this.failure = error;
		this.notify();
		this.resolveDone();
	}

	private notify(): void {
		const wake = this.wake;
		this.wake = null;
		wake?.();
	}
}

/**
 * Reads the cache headers to determine how long this response can be
 * reused, in milliseconds, up to the prefetch time limit.
 */
function usableFor(headers: Headers): number {
	const directives = (headers.get("Cache-Control") ?? "")
		.toLowerCase()
		.split(",")
		.map((directive) => directive.trim());
	if (directives.includes("no-store") || directives.includes("no-cache")) {
		return 0;
	}
	for (const directive of directives) {
		const [name, value] = directive.split("=");
		if (name?.trim() !== "max-age" || value === undefined) continue;
		const seconds = Number.parseInt(value.trim(), 10);
		if (Number.isFinite(seconds)) return Math.min(PREFETCH_TTL, seconds * 1000);
	}
	return PREFETCH_TTL;
}

/**
 * Keeps a limited number of prefetched pages briefly in this document.
 *
 * A page can be reused only if the requested URL and signal values match
 * those used to render it.
 */
export class PrefetchCache {
	private readonly entries = new Map<string, PageRequest>();

	/** Listens for cache-clear events until `lifetime` is aborted. */
	listen(lifetime: AbortSignal): void {
		window.addEventListener(INVALIDATE_PREFETCHES_EVENT, () => this.clear(), {
			signal: lifetime,
		});
	}

	/**
	 * Loads `url` early using the signal values in `body`. Reuses an existing
	 * matching request and skips new requests if too many are loading.
	 * Returns true if a matching prefetch exists or has just started.
	 */
	prefetch(url: string, body: string): boolean {
		const existing = this.entries.get(url);
		if (existing !== undefined) {
			if (existing.body === body && existing.usable) return true;
			this.drop(url);
		}
		let loading = 0;
		for (const entry of this.entries.values()) {
			if (!entry.isFinished && !entry.failed) loading += 1;
		}
		if (loading >= MAX_LOADING) return false;

		const request = new PageRequest(url, body, MAX_BUFFERED);
		this.entries.set(url, request);
		void request.done.then(() => {
			if (request.failed && this.entries.get(url) === request) {
				this.entries.delete(url);
			}
		});
		while (this.entries.size > MAX_PREFETCHES) {
			const oldest = this.entries.keys().next().value;
			if (oldest === undefined) break;
			this.drop(oldest);
		}
		// Remove this entry after the time limit if it is still in the cache.
		setTimeout(() => {
			if (this.entries.get(url) === request) this.drop(url);
		}, PREFETCH_TTL);
		return true;
	}

	/**
	 * Removes the cached request for `url` and returns it if it is still
	 * usable and its signal values match `body`. Otherwise, cancels the
	 * request and returns `null`.
	 */
	take(url: string, body: string): PageRequest | null {
		const request = this.entries.get(url);
		if (request === undefined) return null;
		this.entries.delete(url);
		if (request.body !== body || !request.usable) {
			request.abort();
			return null;
		}
		return request;
	}

	clear(): void {
		for (const url of [...this.entries.keys()]) this.drop(url);
	}

	private drop(url: string): void {
		this.entries.get(url)?.abort();
		this.entries.delete(url);
	}
}
