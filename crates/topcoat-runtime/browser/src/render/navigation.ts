import type { Runtime } from "../runtime";
import { newRender, type RenderToken } from "./frames";
import { PageRequest, type PageResponseHead, PrefetchCache } from "./prefetch";

/**
 * Enables runtime navigation on a link. The value sets when to prefetch:
 * `never`, `intent`, or `viewport`.
 */
export const LINK_ATTRIBUTE = "data-topcoat-link";

/** The hover delay before loading a linked page, in milliseconds. */
const INTENT_DELAY = 80;
/** The delay after scrolling stops before saving the position, in milliseconds. */
const SCROLL_SAVE_DELAY = 150;
/** The property used to store runtime data in history state. */
const STATE_KEY = "topcoat";

/**
 * Whether to add a history entry, replace the current entry, or open an
 * existing entry after the user goes back or forward.
 */
type HistoryMode = "push" | "replace" | "traverse";

type ScrollPosition = { x: number; y: number };

/** Script types to check when deciding whether a page needs new scripts. */
const SCRIPT_TYPES = new Set([
	"",
	"module",
	"text/javascript",
	"application/javascript",
]);

/**
 * Opens pages by updating the current document.
 *
 * Handles links marked for runtime navigation and back or forward visits
 * to its history entries. It asks the server to render the requested page
 * with the current signal values. The current page stays visible until
 * the new page's HTML arrives. After replacing the page, it applies any
 * live updates that follow. Starting another navigation stops the earlier
 * one from changing the document.
 *
 * Links can load pages before the user follows them. Navigation reuses a
 * matching prefetch request, even if its response is still arriving.
 */
export class NavigationController {
	private readonly controller = new AbortController();
	private readonly prefetches = new PrefetchCache();
	/** The request for the most recently opened page. */
	private current: PageRequest | null = null;
	/** The navigation still waiting to replace the displayed document. */
	private uncommitted: PageRequest | null = null;
	private generation = 0;
	/** Resolves when the most recent navigation finishes receiving its response. */
	private streaming: Promise<void> | null = null;
	/** The current page's URL with the # fragment removed. */
	private documentUrl = "";
	/** Scripts already loaded in this document. */
	private readonly scripts = new Set<string>();
	/** Whether this controller is responsible for restoring scroll positions. */
	private managesScroll = false;
	private scrollTimer: ReturnType<typeof setTimeout> | null = null;
	private hovered: HTMLAnchorElement | null = null;
	private hoverTimer: ReturnType<typeof setTimeout> | null = null;
	private viewport: IntersectionObserver | null = null;

	constructor(private readonly runtime: Runtime) {}

	/** Listens for link clicks, back and forward visits, and prefetch triggers. */
	start(): void {
		this.documentUrl = documentUrl(location.href);
		this.prefetches.listen(this.controller.signal);
		for (const key of scriptKeys(document, location.href)) {
			this.scripts.add(key);
		}
		// Restore the saved scroll position when the user reloads this page.
		const scroll = savedScroll(history.state);
		if (scroll !== null) {
			this.manageScroll();
			window.scrollTo(scroll.x, scroll.y);
		}

		const signal = this.controller.signal;
		document.addEventListener("click", (event) => this.onClick(event), {
			signal,
		});
		window.addEventListener("popstate", () => this.onPopState(), { signal });
		window.addEventListener("scroll", () => this.scheduleScrollSave(), {
			passive: true,
			signal,
		});
		window.addEventListener("pagehide", () => this.saveScroll(), { signal });
		window.addEventListener(
			"pageshow",
			(event) => {
				if (event.persisted) this.prefetches.clear();
			},
			{ signal },
		);
		document.addEventListener("mouseover", (event) => this.onHover(event), {
			signal,
		});
		document.addEventListener(
			"mouseout",
			(event) => {
				const to = event.relatedTarget;
				if (to instanceof Node && this.hovered?.contains(to)) return;
				this.cancelHover();
			},
			{ signal },
		);
		document.addEventListener(
			"focusin",
			(event) => {
				const link = findLink(event);
				if (link !== null && policy(link) === "intent") this.prefetch(link);
			},
			{ signal },
		);
		document.addEventListener(
			"touchstart",
			(event) => {
				const link = findLink(event);
				if (link !== null && policy(link) === "intent") this.prefetch(link);
			},
			{ passive: true, signal },
		);
	}

	dispose(): void {
		this.controller.abort();
		this.prefetches.clear();
		this.current?.abort();
		this.viewport?.disconnect();
		this.cancelHover();
	}

	/**
	 * Resolves when the latest navigation finishes receiving its response.
	 * Returns `null` if no navigation is waiting for response data.
	 */
	get pending(): Promise<void> | null {
		return this.streaming;
	}

	/** Loads the linked page when visible. Stops watching when `lifetime` aborts. */
	observe(link: HTMLAnchorElement, lifetime: AbortSignal): void {
		if (typeof IntersectionObserver === "undefined" || lifetime.aborted) {
			return;
		}
		this.viewport ??= new IntersectionObserver((entries) => {
			for (const entry of entries) {
				if (!entry.isIntersecting) continue;
				const target = entry.target as HTMLAnchorElement;
				// If loading was skipped, retry when the link comes back into view.
				if (this.prefetch(target)) this.viewport?.unobserve(target);
			}
		});
		const viewport = this.viewport;
		viewport.observe(link);
		lifetime.addEventListener("abort", () => viewport.unobserve(link), {
			once: true,
		});
	}

	/**
	 * Loads the linked page early using the current signal values.
	 * Returns true if a prefetch exists, has started, or is unnecessary.
	 */
	prefetch(link: HTMLAnchorElement): boolean {
		const url = this.destination(link);
		if (url === null) return true;
		if (savesData()) return false;
		const target = documentUrl(url.href);
		if (target === documentUrl(location.href)) return true;
		return this.prefetches.prefetch(target, this.runtime.page.signalBody());
	}

	/**
	 * Opens `url`. For back or forward visits, `scroll` gives the saved
	 * position to return to.
	 */
	async navigate(
		url: URL,
		mode: HistoryMode,
		scroll: ScrollPosition | null = null,
	): Promise<void> {
		this.generation += 1;
		const generation = this.generation;
		this.current?.abort();
		const target = documentUrl(url.href);
		const body = this.runtime.page.signalBody();
		const request =
			this.prefetches.take(target, body) ?? new PageRequest(target, body);
		this.current = request;
		this.uncommitted = request;

		let finish!: () => void;
		const streaming = new Promise<void>((resolve) => {
			finish = resolve;
		});
		this.streaming = streaming;
		const isCurrent = () =>
			generation === this.generation && !this.controller.signal.aborted;
		try {
			await this.follow(request, url, mode, scroll, isCurrent);
		} finally {
			if (this.uncommitted === request) this.uncommitted = null;
			if (this.streaming === streaming) this.streaming = null;
			finish();
		}
	}

	/** Reads the response and displays the page and its live updates. */
	private async follow(
		request: PageRequest,
		url: URL,
		mode: HistoryMode,
		scroll: ScrollPosition | null,
		isCurrent: () => boolean,
	): Promise<void> {
		let head: PageResponseHead;
		try {
			head = await request.head;
		} catch {
			if (isCurrent()) load(url, mode);
			return;
		}
		if (!isCurrent()) return;
		if (!head.frames) {
			load(url, mode);
			return;
		}
		// Keep the original fragment if the redirect does not supply one.
		const destination = new URL(head.url, location.href);
		if (destination.hash === "") destination.hash = url.hash;

		const { page } = this.runtime;
		const render = newRender();
		let committed = false;
		try {
			for await (const frame of request.frames()) {
				if (!isCurrent()) return;
				switch (frame.t) {
					case "snapshot": {
						if (committed) {
							page.replaceContent(frame.html, render);
							break;
						}
						const next = new DOMParser().parseFromString(
							frame.html,
							"text/html",
						);
						if (!this.canCommit(next, destination)) {
							request.abort();
							load(destination, mode);
							return;
						}
						this.commit(next, destination, mode, scroll, render);
						if (this.uncommitted === request) this.uncommitted = null;
						committed = true;
						break;
					}
					case "swap":
						if (committed) page.applySwap(frame.region, frame.html, render);
						break;
					case "redirect": {
						request.abort();
						const location = new URL(frame.location, destination);
						if (location.origin !== window.location.origin) {
							window.location.assign(location.href);
							return;
						}
						const next = committed || mode === "traverse" ? "replace" : mode;
						void this.navigate(location, next);
						return;
					}
					case "error":
						throw new Error(`Navigation render failed: ${frame.status}`);
				}
			}
			// A response without a snapshot leaves nothing to show, so the
			// browser loads the destination instead.
			if (!committed && isCurrent()) load(destination, mode);
		} catch (error) {
			if (!isCurrent()) return;
			if (!committed) {
				load(destination, mode);
				return;
			}
			this.runtime.reportError(error);
		}
	}

	/**
	 * Checks whether `next` can be displayed without a full reload. The
	 * doctype and base element must match, and every script must already be
	 * loaded. Scripts inserted during a page update do not run.
	 */
	private canCommit(next: Document, url: URL): boolean {
		const doctype = (doc: Document) => {
			const type = doc.doctype;
			return JSON.stringify(type && [type.name, type.publicId, type.systemId]);
		};
		const base = (doc: Document) =>
			doc.querySelector("base")?.outerHTML ?? null;
		return (
			doctype(document) === doctype(next) &&
			base(document) === base(next) &&
			scriptKeys(next, url.href).every((key) => this.scripts.has(key))
		);
	}

	/** Displays `next` and updates the URL, history, scroll position, and focus. */
	private commit(
		next: Document,
		url: URL,
		mode: HistoryMode,
		scroll: ScrollPosition | null,
		render: RenderToken,
	): void {
		// Discard pages loaded with the old page's signals and server state.
		this.prefetches.clear();
		this.cancelHover();
		this.manageScroll();
		switch (mode) {
			case "push":
				this.saveScroll();
				history.pushState(withScroll(null, null), "", url.href);
				break;
			case "replace":
				history.replaceState(withScroll(history.state, null), "", url.href);
				break;
			case "traverse":
				if (url.href !== location.href) {
					history.replaceState(history.state, "", url.href);
				}
				break;
		}
		this.documentUrl = documentUrl(url.href);
		this.runtime.page.replaceDocument(next, render);

		const target = fragmentTarget(url);
		if (mode === "traverse" && scroll !== null) {
			window.scrollTo(scroll.x, scroll.y);
		} else if (target !== null) {
			target.scrollIntoView();
		} else {
			window.scrollTo(0, 0);
		}
		resetFocus(target);
	}

	private onClick(event: MouseEvent): void {
		if (
			event.defaultPrevented ||
			event.button !== 0 ||
			event.metaKey ||
			event.ctrlKey ||
			event.shiftKey ||
			event.altKey
		) {
			return;
		}
		const link = findLink(event);
		if (link === null) return;
		const url = this.destination(link);
		if (url === null) return;
		event.preventDefault();
		// Reuse the history entry when opening the current URL again.
		void this.navigate(url, url.href === location.href ? "replace" : "push");
	}

	private onPopState(): void {
		if (this.scrollTimer !== null) {
			// Do not save the previous page's scroll position in this entry.
			clearTimeout(this.scrollTimer);
			this.scrollTimer = null;
		}
		const state: unknown = history.state;
		// Leave history entries with application state to the application.
		if (state !== null && !hasEntry(state)) return;
		const url = new URL(location.href);
		const scroll = savedScroll(state);
		if (documentUrl(url.href) !== this.documentUrl) {
			void this.navigate(url, "traverse", scroll);
			return;
		}
		// Returning to the displayed page cancels a pending navigation, but
		// keeps any request already streaming updates into this document.
		if (this.uncommitted !== null) {
			this.generation += 1;
			this.uncommitted.abort();
			this.uncommitted = null;
		}
		// The URL points to another fragment on the same page.
		if (!this.managesScroll) return;
		if (scroll !== null) {
			window.scrollTo(scroll.x, scroll.y);
		} else {
			fragmentTarget(url)?.scrollIntoView();
		}
	}

	private onHover(event: MouseEvent): void {
		const found = findLink(event);
		const link = found !== null && policy(found) === "intent" ? found : null;
		if (link === this.hovered) return;
		this.cancelHover();
		if (link === null) return;
		this.hovered = link;
		this.hoverTimer = setTimeout(() => {
			this.hoverTimer = null;
			this.prefetch(link);
		}, INTENT_DELAY);
	}

	private cancelHover(): void {
		if (this.hoverTimer !== null) clearTimeout(this.hoverTimer);
		this.hoverTimer = null;
		this.hovered = null;
	}

	/**
	 * Returns the link's URL if runtime navigation can handle it.
	 * Returns `null` to let the browser handle the link as usual.
	 */
	private destination(link: HTMLAnchorElement): URL | null {
		if (link.hasAttribute("download") || !link.hasAttribute("href")) {
			return null;
		}
		const target = effectiveTarget(link).toLowerCase();
		if (target !== "" && target !== "_self") return null;
		let url: URL;
		try {
			url = new URL(link.href, location.href);
		} catch {
			return null;
		}
		if (url.origin !== location.origin) return null;
		if (url.protocol !== "http:" && url.protocol !== "https:") return null;
		// Let the browser scroll to fragments on the current page.
		if (
			url.href.includes("#") &&
			documentUrl(url.href) === documentUrl(location.href)
		) {
			return null;
		}
		return url;
	}

	/**
	 * Disables automatic browser scroll restoration. This controller then
	 * restores the saved position when the user goes back or forward.
	 */
	private manageScroll(): void {
		if (this.managesScroll) return;
		this.managesScroll = true;
		history.scrollRestoration = "manual";
		this.saveScroll();
	}

	private scheduleScrollSave(): void {
		if (!this.managesScroll) return;
		if (this.scrollTimer !== null) clearTimeout(this.scrollTimer);
		this.scrollTimer = setTimeout(() => {
			this.scrollTimer = null;
			this.saveScroll();
		}, SCROLL_SAVE_DELAY);
	}

	/** Records where the page is scrolled in the current history entry. */
	private saveScroll(): void {
		if (!this.managesScroll) return;
		if (this.scrollTimer !== null) clearTimeout(this.scrollTimer);
		this.scrollTimer = null;
		const scroll = { x: window.scrollX, y: window.scrollY };
		history.replaceState(withScroll(history.state, scroll), "");
	}
}

/** Removes the # fragment from `url`, if present. */
function documentUrl(url: string): string {
	const index = url.indexOf("#");
	return index === -1 ? url : url.slice(0, index);
}

/** Finds the event's nearest anchor if it is marked for runtime navigation. */
function findLink(event: Event): HTMLAnchorElement | null {
	for (const target of event.composedPath()) {
		if (target instanceof HTMLAnchorElement) {
			return target.hasAttribute(LINK_ATTRIBUTE) ? target : null;
		}
	}
	return null;
}

/**
 * Returns the browsing context `link` opens in: its own `target`, or the
 * target of the document's first `<base>` element that sets one.
 */
function effectiveTarget(link: HTMLAnchorElement): string {
	if (link.hasAttribute("target")) return link.getAttribute("target") ?? "";
	return (
		link.ownerDocument.querySelector("base[target]")?.getAttribute("target") ??
		""
	);
}

function policy(link: HTMLAnchorElement): string | null {
	return link.getAttribute(LINK_ATTRIBUTE);
}

/** Returns whether the browser's data-saving setting is enabled. */
function savesData(): boolean {
	const connection = (
		navigator as Navigator & { connection?: { saveData?: boolean } }
	).connection;
	return connection?.saveData === true;
}

/** Builds comparison keys for executable scripts in `doc`. */
function scriptKeys(doc: Document, base: string): string[] {
	const keys: string[] = [];
	for (const script of Array.from(doc.scripts)) {
		const type = script.getAttribute("type")?.trim().toLowerCase() ?? "";
		if (!SCRIPT_TYPES.has(type)) continue;
		const kind = type === "module" ? "module" : "classic";
		const src = script.getAttribute("src");
		if (src !== null) {
			keys.push(`${kind} src ${new URL(src, base).href}`);
		} else {
			keys.push(`${kind} inline ${script.text}`);
		}
	}
	return keys;
}

/** Opens the URL with a normal browser page load. */
function load(url: URL, mode: HistoryMode): void {
	if (mode === "push") {
		location.assign(url.href);
	} else {
		location.replace(url.href);
	}
}

/** Finds the element named by the URL's # fragment. */
function fragmentTarget(url: URL): HTMLElement | null {
	let id: string;
	try {
		id = decodeURIComponent(url.hash.slice(1));
	} catch {
		return null;
	}
	if (id === "") return null;
	const element = document.getElementById(id);
	if (element !== null) return element;
	for (const named of Array.from(document.getElementsByName(id))) {
		if (named instanceof HTMLAnchorElement) return named;
	}
	return null;
}

/**
 * Moves focus to the fragment target or the new page's body so keyboard
 * navigation starts there. An element with `autofocus` takes priority.
 */
function resetFocus(target: HTMLElement | null): void {
	const autofocus = document.querySelector("[autofocus]");
	if (autofocus instanceof HTMLElement) {
		autofocus.focus();
		return;
	}
	const element = target ?? document.body;
	const temporary = !element.hasAttribute("tabindex");
	if (temporary) element.setAttribute("tabindex", "-1");
	element.focus({ preventScroll: true });
	if (temporary) {
		element.addEventListener(
			"blur",
			() => element.removeAttribute("tabindex"),
			{
				once: true,
			},
		);
	}
}

function hasEntry(state: unknown): state is Record<string, unknown> {
	return typeof state === "object" && state !== null && STATE_KEY in state;
}

/** Reads the stored scroll position from a history entry. */
function savedScroll(state: unknown): ScrollPosition | null {
	if (!hasEntry(state)) return null;
	const entry = state[STATE_KEY] as { scroll?: ScrollPosition | null };
	return entry.scroll ?? null;
}

/** Copies the history state and sets the runtime's saved scroll position. */
function withScroll(
	state: unknown,
	scroll: ScrollPosition | null,
): Record<string, unknown> {
	const own = typeof state === "object" && state !== null ? state : {};
	return { ...own, [STATE_KEY]: { scroll } };
}
