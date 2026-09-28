import type { Runtime } from "../runtime";
import { newRender, type RenderToken } from "./frames";
import { PageRequest, type PageResponseHead, PrefetchCache } from "./prefetch";

/**
 * Opts an anchor into runtime navigation. Its value is the prefetch policy:
 * `never`, `intent`, or `viewport`.
 */
export const LINK_ATTRIBUTE = "data-topcoat-link";

/** How long the pointer rests on a link before it is prefetched. */
const INTENT_DELAY = 80;
/** How long scrolling pauses before the position is saved to history. */
const SCROLL_SAVE_DELAY = 150;
/** The key of the runtime's entry in a history entry's state. */
const STATE_KEY = "topcoat";

/**
 * How a navigation changes the session history: it adds an entry, replaces
 * the current one, or shows an entry the user traversed to.
 */
type HistoryMode = "push" | "replace" | "traverse";

type ScrollPosition = { x: number; y: number };

/** The script types a browser executes. */
const SCRIPT_TYPES = new Set([
	"",
	"module",
	"text/javascript",
	"application/javascript",
]);

/**
 * Owns navigations between pages of the document.
 *
 * A click on an opted-in link, or a traversal of an entry the controller
 * created, requests the destination as a page rerun with the document's
 * signal values. The current page stays visible until the destination's
 * snapshot arrives. The snapshot then replaces the document, and later
 * frames update its live regions. A newer navigation stops an older one
 * from committing or applying further frames.
 *
 * Links can prefetch their destination ahead of a click. A navigation takes
 * over a matching prefetch, including one still streaming.
 */
export class NavigationController {
	private readonly controller = new AbortController();
	private readonly prefetches = new PrefetchCache();
	/** The page request of the latest navigation. */
	private current: PageRequest | null = null;
	private generation = 0;
	/** Settles when the latest navigation's response has ended. */
	private streaming: Promise<void> | null = null;
	/** The URL of the displayed document, without its fragment. */
	private documentUrl = "";
	/** The scripts the document has loaded. */
	private readonly scripts = new Set<string>();
	/** Whether the controller restores the scroll position of its entries. */
	private managesScroll = false;
	private scrollTimer: ReturnType<typeof setTimeout> | null = null;
	private hovered: HTMLAnchorElement | null = null;
	private hoverTimer: ReturnType<typeof setTimeout> | null = null;
	private viewport: IntersectionObserver | null = null;

	constructor(private readonly runtime: Runtime) {}

	/** Starts handling link clicks, history traversals, and prefetching. */
	start(): void {
		this.documentUrl = documentUrl(location.href);
		this.prefetches.listen(this.controller.signal);
		for (const key of scriptKeys(document, location.href)) {
			this.scripts.add(key);
		}
		// A reload of an entry this controller scrolled keeps its position.
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
	 * Settles once the latest navigation's response has ended, or is `null`
	 * when no navigation is streaming.
	 */
	get pending(): Promise<void> | null {
		return this.streaming;
	}

	/** Prefetches `link` once it becomes visible, until `lifetime` aborts. */
	observe(link: HTMLAnchorElement, lifetime: AbortSignal): void {
		if (typeof IntersectionObserver === "undefined" || lifetime.aborted) {
			return;
		}
		this.viewport ??= new IntersectionObserver((entries) => {
			for (const entry of entries) {
				if (!entry.isIntersecting) continue;
				const target = entry.target as HTMLAnchorElement;
				// A skipped prefetch tries again when the link reappears.
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
	 * Prefetches the destination of `link` with the current signal values.
	 * Returns whether the destination is prefetched or needs no prefetch.
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
	 * Navigates to `url`. `scroll` is the position to restore when showing
	 * a traversed entry.
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
			if (this.streaming === streaming) this.streaming = null;
			finish();
		}
	}

	/** Reads a navigation's response, committing its snapshot. */
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
		// A redirect keeps the fragment of the requested URL.
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
	 * Checks that `next` can replace the document in place: it uses the
	 * same doctype and base URL, and loads no script the document has not
	 * loaded, as inserted scripts do not run.
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

	/** Makes `next` the displayed document at `url`. */
	private commit(
		next: Document,
		url: URL,
		mode: HistoryMode,
		scroll: ScrollPosition | null,
		render: RenderToken,
	): void {
		// Prefetches used the previous page's signals and server state.
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
		// Following a link to the current URL replaces its entry.
		void this.navigate(url, url.href === location.href ? "replace" : "push");
	}

	private onPopState(): void {
		if (this.scrollTimer !== null) {
			// The pending position belongs to the entry the user left.
			clearTimeout(this.scrollTimer);
			this.scrollTimer = null;
		}
		const state: unknown = history.state;
		// Entries with state of their own belong to the application.
		if (state !== null && !hasEntry(state)) return;
		const url = new URL(location.href);
		const scroll = savedScroll(state);
		if (documentUrl(url.href) !== this.documentUrl) {
			void this.navigate(url, "traverse", scroll);
			return;
		}
		// A fragment change within the displayed document.
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
	 * Returns the URL `link` navigates to at runtime, or `null` when the
	 * browser handles the link itself.
	 */
	private destination(link: HTMLAnchorElement): URL | null {
		if (link.hasAttribute("download") || !link.hasAttribute("href")) {
			return null;
		}
		if (link.target !== "" && link.target !== "_self") return null;
		let url: URL;
		try {
			url = new URL(link.href, location.href);
		} catch {
			return null;
		}
		if (url.origin !== location.origin) return null;
		if (url.protocol !== "http:" && url.protocol !== "https:") return null;
		// Same-document fragment links keep normal fragment navigation.
		if (
			url.href.includes("#") &&
			documentUrl(url.href) === documentUrl(location.href)
		) {
			return null;
		}
		return url;
	}

	/**
	 * Takes over scroll restoration, so traversing to one of this
	 * document's entries restores the position saved with it.
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

	/** Saves the scroll position in the current history entry. */
	private saveScroll(): void {
		if (!this.managesScroll) return;
		if (this.scrollTimer !== null) clearTimeout(this.scrollTimer);
		this.scrollTimer = null;
		const scroll = { x: window.scrollX, y: window.scrollY };
		history.replaceState(withScroll(history.state, scroll), "");
	}
}

/** Returns `url` without its fragment. */
function documentUrl(url: string): string {
	const index = url.indexOf("#");
	return index === -1 ? url : url.slice(0, index);
}

/** Finds the innermost anchor the event passed through, if it opted in. */
function findLink(event: Event): HTMLAnchorElement | null {
	for (const target of event.composedPath()) {
		if (target instanceof HTMLAnchorElement) {
			return target.hasAttribute(LINK_ATTRIBUTE) ? target : null;
		}
	}
	return null;
}

function policy(link: HTMLAnchorElement): string | null {
	return link.getAttribute(LINK_ATTRIBUTE);
}

/** Checks whether the user asked the browser to reduce data usage. */
function savesData(): boolean {
	const connection = (
		navigator as Navigator & { connection?: { saveData?: boolean } }
	).connection;
	return connection?.saveData === true;
}

/** Identifies the scripts in `doc` that the browser executes. */
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

/** Leaves the navigation to the browser. */
function load(url: URL, mode: HistoryMode): void {
	if (mode === "push") {
		location.assign(url.href);
	} else {
		location.replace(url.href);
	}
}

/** Returns the element a URL's fragment points at. */
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
 * Starts sequential focus navigation at the fragment target or the top of
 * the new page, unless an element asks for focus.
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

/** Returns the scroll position saved in a history entry's state. */
function savedScroll(state: unknown): ScrollPosition | null {
	if (!hasEntry(state)) return null;
	const entry = state[STATE_KEY] as { scroll?: ScrollPosition | null };
	return entry.scroll ?? null;
}

/** Returns `state` with the runtime's entry holding `scroll`. */
function withScroll(
	state: unknown,
	scroll: ScrollPosition | null,
): Record<string, unknown> {
	const own = typeof state === "object" && state !== null ? state : {};
	return { ...own, [STATE_KEY]: { scroll } };
}
