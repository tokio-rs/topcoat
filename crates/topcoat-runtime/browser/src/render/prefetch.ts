import { FRAMES_MEDIA_TYPE, readFrames, type ServerMessage } from "./frames";
import { RUNTIME_HEADER } from "./request";

/** How long a prefetched page stays usable. */
const PREFETCH_TTL = 30_000;
/** The most prefetched pages kept at once; the oldest is dropped first. */
const MAX_PREFETCHES = 8;
/** The most prefetches loading at once; further prefetches are skipped. */
const MAX_LOADING = 2;
/** The most frame text a prefetch buffers before it is dropped. */
const MAX_BUFFERED = 1024 * 1024;

/** Discards every prefetched page, for example after a procedure call. */
export const INVALIDATE_PREFETCHES_EVENT = "topcoat:invalidate-prefetches";

/** Signals that server state may have changed since pages were prefetched. */
export function invalidatePrefetches(): void {
	if (typeof window === "undefined") return;
	window.dispatchEvent(new Event(INVALIDATE_PREFETCHES_EVENT));
}

/** What the response to a page request turned out to be. */
export type PageResponseHead = {
	/** The URL the response came from, after any HTTP redirects. */
	url: string;
	/** Whether the response carries render frames rather than other content. */
	frames: boolean;
	/** Until when, in `performance.now()` time, a prefetch may be used. */
	usableUntil: number;
};

/**
 * Renders a page at a URL with a set of signal values, the way a page
 * rerun does, and keeps its frames until they are read.
 *
 * Reading starts from the first frame, whether the frames arrived before
 * or after reading began, so a prefetched page can be handed to a
 * navigation that continues the same response.
 */
export class PageRequest {
	readonly head: Promise<PageResponseHead>;
	/** The head once it has arrived. */
	private arrived: PageResponseHead | null = null;
	private readonly controller = new AbortController();
	private readonly buffer: ServerMessage[] = [];
	private buffered = 0;
	/** The buffer limit while nobody reads, or `null` without one. */
	private limit: number | null;
	private finished = false;
	private failure: unknown = null;
	private wake: (() => void) | null = null;
	/** Settled once the response has ended, failed, or been aborted. */
	readonly done: Promise<void>;
	private resolveDone!: () => void;

	constructor(
		/** The requested URL, without a fragment. */
		readonly url: string,
		/** The JSON request body carrying the signal values. */
		readonly body: string,
		limit: number | null = null,
	) {
		this.limit = limit;
		this.done = new Promise((resolve) => {
			this.resolveDone = resolve;
		});
		this.head = this.start();
		// A caller that never reads the head must not see an unhandled rejection.
		this.head.catch(() => undefined);
	}

	get isFinished(): boolean {
		return this.finished;
	}

	get failed(): boolean {
		return this.failure !== null;
	}

	/**
	 * Whether the response can still be used. A response still on its way
	 * is as fresh as a new request.
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

	/** Yields every frame of the response, waiting for those still to come. */
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
			// Other content is loaded by the browser instead.
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
 * Returns how long a response may be reused, in milliseconds, honoring
 * the response's cache restrictions.
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
 * A small, short-lived set of prefetched pages, private to this document.
 *
 * A prefetch is tied to the signal values it was rendered with, and can
 * only be used by a navigation to the same URL with the same values.
 */
export class PrefetchCache {
	private readonly entries = new Map<string, PageRequest>();

	/** Clears the cache on each invalidation until `lifetime` aborts. */
	listen(lifetime: AbortSignal): void {
		window.addEventListener(INVALIDATE_PREFETCHES_EVENT, () => this.clear(), {
			signal: lifetime,
		});
	}

	/**
	 * Starts prefetching `url` with the signal values in `body`, unless an
	 * equivalent prefetch exists or too many are already loading. Returns
	 * whether the page is prefetched.
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
		// Expire entries that nobody claims.
		setTimeout(() => {
			if (this.entries.get(url) === request) this.drop(url);
		}, PREFETCH_TTL);
		return true;
	}

	/**
	 * Hands over the prefetch of `url` if it was rendered with the signal
	 * values in `body` and is still usable. The prefetch leaves the cache
	 * either way.
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
