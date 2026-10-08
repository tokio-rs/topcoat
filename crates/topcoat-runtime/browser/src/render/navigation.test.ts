// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { flushEffects } from "../reactivity";
import { Runtime } from "../runtime";
import { F64 } from "../surrogate";
import type { ServerMessage } from "./frames";
import { invalidatePrefetches } from "./prefetch";
import { RUNTIME_HEADER } from "./request";

const FRAMES = "application/x-ndjson";

let runtime: Runtime | null = null;

beforeEach(() => {
	history.replaceState(null, "", "/");
	history.scrollRestoration = "auto";
	vi.spyOn(location, "assign").mockImplementation(() => undefined);
	vi.spyOn(location, "replace").mockImplementation(() => undefined);
});

afterEach(() => {
	runtime?.navigation.dispose();
	runtime?.page.dispose();
	runtime = null;
	vi.unstubAllGlobals();
	vi.restoreAllMocks();
});

const declaration = (id: string, value: number) =>
	`<!--::topcoat::signal({"t":"signal","id":"${id}","v":${value}})-->`;

/** Converts a frame to JSON followed by a newline, matching the server format. */
const line = (frame: ServerMessage) => `${JSON.stringify(frame)}\n`;

/** Builds an HTML document with the given title and body. */
const page = (title: string, body: string) =>
	`<html><head><title>${title}</title></head><body>${body}</body></html>`;

/** Creates a response with a single HTML snapshot frame. */
function snapshot(html: string, headers: Record<string, string> = {}) {
	return new Response(line({ t: "snapshot", html }), {
		headers: { "Content-Type": FRAMES, ...headers },
	});
}

/** Creates a response that the test can send frames through as needed. */
function streamed() {
	let controller!: ReadableStreamDefaultController<Uint8Array>;
	const body = new ReadableStream<Uint8Array>({
		start(c) {
			controller = c;
		},
	});
	const encoder = new TextEncoder();
	return {
		response: new Response(body, { headers: { "Content-Type": FRAMES } }),
		push: (frame: ServerMessage) =>
			controller.enqueue(encoder.encode(line(frame))),
		close: () => controller.close(),
	};
}

type Call = { url: string; init: RequestInit };

/**
 * Records each fetch request and calls `respond` with its URL path to
 * get the response.
 */
function stubFetch(respond: (path: string) => Response | Promise<Response>) {
	const calls: Call[] = [];
	vi.stubGlobal(
		"fetch",
		async (input: RequestInfo | URL, init: RequestInit) => {
			const url = String(input);
			calls.push({ url, init });
			return respond(new URL(url).pathname);
		},
	);
	return calls;
}

/** Sets up a document with `body` and starts the runtime. */
function mount(body: string): Runtime {
	document.documentElement.innerHTML = `<head><title>Home</title></head><body>${body}</body>`;
	runtime = new Runtime();
	runtime.start(document);
	return runtime;
}

/** Lets queued requests, response frames, and page updates run. */
async function settle(): Promise<void> {
	for (let i = 0; i < 5; i += 1) {
		flushEffects();
		await new Promise((resolve) => setTimeout(resolve, 0));
	}
	flushEffects();
}

/**
 * Sends a click to `element` and reports whether the runtime handled it.
 * Prevents the browser from following the link itself.
 */
function click(element: Element, init: MouseEventInit = {}): boolean {
	const event = new MouseEvent("click", {
		bubbles: true,
		cancelable: true,
		composed: true,
		button: 0,
		...init,
	});
	let handled = false;
	// This listener runs after the runtime's click handler.
	const suppress = (seen: Event) => {
		handled = seen.defaultPrevented;
		seen.preventDefault();
	};
	document.addEventListener("click", suppress, { once: true });
	element.dispatchEvent(event);
	document.removeEventListener("click", suppress);
	return handled;
}

function link(selector = "a"): HTMLAnchorElement {
	return document.querySelector(selector) as HTMLAnchorElement;
}

it("follows a link in place, keeping the signals the destination declares again", async () => {
	const calls = stubFetch(() =>
		snapshot(page("Next", `${declaration("a", 1)}<p>next page</p>`)),
	);
	const rt = mount(
		`${declaration("a", 1)}${declaration("b", 2)}<a data-topcoat-link="intent" href="/next">next</a>`,
	);
	rt.context.signal("a").set(new F64(5));

	const handled = click(link());
	await settle();

	expect(handled).toBe(true);
	expect(calls).toHaveLength(1);
	const [call] = calls as [Call];
	expect(call.url).toBe(`${location.origin}/next`);
	expect(call.init.method).toBe("POST");
	const headers = call.init.headers as Record<string, string>;
	expect(headers[RUNTIME_HEADER]).toBe("true");
	expect(headers.Accept).toBe(FRAMES);
	expect(JSON.parse(call.init.body as string)).toEqual({
		signals: { a: 5, b: 2 },
	});

	expect(location.pathname).toBe("/next");
	expect(document.title).toBe("Next");
	expect(document.body.textContent).toContain("next page");
	// Keep the browser's signal value instead of the new page's initial value.
	expect(rt.registry.read("a")).toEqual(new F64(5));
	expect(rt.registry.has("b")).toBe(false);
});

it.each([
	["an ordinary anchor", `<a href="/next">x</a>`, {}],
	[
		"a modified click",
		`<a data-topcoat-link="intent" href="/next">x</a>`,
		{ ctrlKey: true },
	],
	[
		"a secondary button",
		`<a data-topcoat-link="intent" href="/next">x</a>`,
		{ button: 1 },
	],
	[
		"another target",
		`<a data-topcoat-link="intent" href="/next" target="_blank">x</a>`,
		{},
	],
	[
		"a download",
		`<a data-topcoat-link="intent" href="/next" download>x</a>`,
		{},
	],
	[
		"another origin",
		`<a data-topcoat-link="intent" href="https://example.com/next">x</a>`,
		{},
	],
	[
		"another scheme",
		`<a data-topcoat-link="intent" href="mailto:a@example.com">x</a>`,
		{},
	],
	[
		"a fragment of the current page",
		`<a data-topcoat-link="intent" href="#section">x</a>`,
		{},
	],
])("leaves %s to the browser", async (_, markup, init) => {
	const calls = stubFetch(() => snapshot(page("Next", "")));
	mount(markup);

	const handled = click(link(), init);
	await settle();

	expect(handled).toBe(false);
	expect(calls).toHaveLength(0);
	expect(location.pathname).toBe("/");
});

it("respects a click the application already canceled", async () => {
	const calls = stubFetch(() => snapshot(page("Next", "")));
	mount(`<a data-topcoat-link="never" href="/next">x</a>`);
	link().addEventListener("click", (event) => event.preventDefault());

	click(link());
	await settle();

	expect(calls).toHaveLength(0);
	expect(location.pathname).toBe("/");
});

it("leaves a link that inherits another target from the base element to the browser", async () => {
	const calls = stubFetch(() => snapshot(page("Next", "")));
	mount(`<a data-topcoat-link="intent" href="/next">x</a>`);
	const base = document.createElement("base");
	base.target = "_blank";
	document.head.append(base);

	link().dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
	const handled = click(link());
	await settle();

	expect(handled).toBe(false);
	expect(calls).toHaveLength(0);
	expect(location.pathname).toBe("/");
});

it("follows a link whose own target overrides the base element's", async () => {
	// The destination keeps the base element, so it can replace the page.
	stubFetch(() =>
		snapshot(
			`<html><head><base target="_blank"><title>Next</title></head><body><p>next page</p></body></html>`,
		),
	);
	mount(`<a data-topcoat-link="never" href="/next" target="_self">x</a>`);
	const base = document.createElement("base");
	base.target = "_blank";
	document.head.append(base);

	const handled = click(link());
	await settle();

	expect(handled).toBe(true);
	expect(location.pathname).toBe("/next");
});

it("loads the destination in the browser when the response ends without a snapshot", async () => {
	stubFetch(() => new Response("", { headers: { "Content-Type": FRAMES } }));
	mount(`<p>home</p><a data-topcoat-link="never" href="/next">x</a>`);

	const handled = click(link());
	await settle();

	expect(handled).toBe(true);
	expect(location.assign).toHaveBeenCalledWith(`${location.origin}/next`);
	expect(location.pathname).toBe("/");
	expect(document.body.textContent).toContain("home");
});

it("keeps the current page until the destination's snapshot arrives, and lets a newer navigation win", async () => {
	const slow = streamed();
	stubFetch((path) =>
		path === "/slow"
			? slow.response
			: snapshot(page("Fast", "<p>fast page</p>")),
	);
	mount(
		`<p>home</p><a id="slow" data-topcoat-link="never" href="/slow">s</a><a id="fast" data-topcoat-link="never" href="/fast">f</a>`,
	);

	click(link("#slow"));
	await settle();
	expect(document.body.textContent).toContain("home");
	expect(location.pathname).toBe("/");

	click(link("#fast"));
	await settle();
	slow.push({ t: "snapshot", html: page("Slow", "<p>slow page</p>") });
	slow.close();
	await settle();

	expect(location.pathname).toBe("/fast");
	expect(document.body.textContent).toContain("fast page");
	expect(document.body.textContent).not.toContain("slow page");
	// The canceled navigation must not trigger a full page load.
	expect(location.assign).not.toHaveBeenCalled();
	expect(location.replace).not.toHaveBeenCalled();
});

it("applies later frames to the destination's live regions", async () => {
	const stream = streamed();
	stubFetch(() => stream.response);
	mount(`<a data-topcoat-link="never" href="/next">x</a>`);

	click(link());
	stream.push({
		t: "snapshot",
		html: page(
			"Next",
			"<!--::topcoat::region::start(1f)--><p>loading</p><!--::topcoat::region::end(1f)-->",
		),
	});
	await settle();
	expect(document.body.textContent).toContain("loading");

	stream.push({ t: "swap", region: "1f", html: "<p>loaded</p>" });
	await settle();

	expect(document.body.textContent).toContain("loaded");
	expect(document.body.textContent).not.toContain("loading");
});

it("loads a destination that is not a view in the browser", async () => {
	stubFetch(
		() =>
			new Response("{}", { headers: { "Content-Type": "application/json" } }),
	);
	mount(`<p>home</p><a data-topcoat-link="never" href="/data">x</a>`);

	click(link());
	await settle();

	expect(location.assign).toHaveBeenCalledWith(`${location.origin}/data`);
	expect(location.pathname).toBe("/");
	expect(document.body.textContent).toContain("home");
});

it("loads a destination that needs a script the document has not loaded in the browser", async () => {
	stubFetch(() =>
		snapshot(
			`<html><head><script src="/other.js"></script></head><body></body></html>`,
		),
	);
	mount(`<a data-topcoat-link="never" href="/next">x</a>`);

	click(link());
	await settle();

	expect(location.assign).toHaveBeenCalledWith(`${location.origin}/next`);
	expect(location.pathname).toBe("/");
});

it("follows a redirect frame in place", async () => {
	const calls = stubFetch((path) =>
		path === "/private"
			? new Response(line({ t: "redirect", location: "/login" }), {
					headers: { "Content-Type": FRAMES },
				})
			: snapshot(page("Login", "<p>log in</p>")),
	);
	mount(`<a data-topcoat-link="never" href="/private">x</a>`);

	click(link());
	await settle();

	expect(calls.map((call) => new URL(call.url).pathname)).toEqual([
		"/private",
		"/login",
	]);
	expect(location.pathname).toBe("/login");
	expect(document.body.textContent).toContain("log in");
	expect(location.assign).not.toHaveBeenCalled();
});

it("shows the previous page again on a traversal and restores its scroll position", async () => {
	stubFetch((path) =>
		path === "/next"
			? snapshot(page("Next", "<p>next page</p>"))
			: snapshot(
					page(
						"Home",
						`<p>home again</p><a data-topcoat-link="never" href="/next">x</a>`,
					),
				),
	);
	mount(`<p>home</p><a data-topcoat-link="never" href="/next">x</a>`);
	const scrollTo = vi.spyOn(window, "scrollTo");
	vi.spyOn(window, "scrollY", "get").mockReturnValue(300);

	click(link());
	await settle();
	expect(location.pathname).toBe("/next");
	expect(history.scrollRestoration).toBe("manual");

	// Simulate pressing the browser's back button.
	history.go(-1);
	await settle();

	expect(location.pathname).toBe("/");
	expect(document.body.textContent).toContain("home again");
	expect(scrollTo).toHaveBeenLastCalledWith(0, 300);
});

it("prefetches a link on hover and uses the prefetch when it is followed", async () => {
	const calls = stubFetch(() => snapshot(page("Next", "<p>next page</p>")));
	mount(`<a data-topcoat-link="intent" href="/next">x</a>`);

	link().dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
	await new Promise((resolve) => setTimeout(resolve, 150));
	expect(calls).toHaveLength(1);

	click(link());
	await settle();

	expect(calls).toHaveLength(1);
	expect(location.pathname).toBe("/next");
	expect(document.body.textContent).toContain("next page");
});

it("prefetches a link as soon as it is focused", async () => {
	const calls = stubFetch(() => snapshot(page("Next", "")));
	mount(`<a data-topcoat-link="intent" href="/next">x</a>`);

	link().dispatchEvent(new FocusEvent("focusin", { bubbles: true }));

	expect(calls).toHaveLength(1);
	expect(location.pathname).toBe("/");
});

it("does not prefetch links whose policy does not ask for it", async () => {
	const calls = stubFetch(() => snapshot(page("Next", "")));
	mount(`<a data-topcoat-link="never" href="/next">x</a>`);

	link().dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
	link().dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
	await new Promise((resolve) => setTimeout(resolve, 150));

	expect(calls).toHaveLength(0);
});

it("prefetches a viewport link once it becomes visible", async () => {
	const observers: ((entries: Partial<IntersectionObserverEntry>[]) => void)[] =
		[];
	vi.stubGlobal(
		"IntersectionObserver",
		class {
			constructor(
				callback: (entries: Partial<IntersectionObserverEntry>[]) => void,
			) {
				observers.push(callback);
			}
			observe() {}
			unobserve() {}
			disconnect() {}
		},
	);
	const calls = stubFetch(() => snapshot(page("Next", "")));
	mount(`<a data-topcoat-link="viewport" href="/next">x</a>`);
	expect(calls).toHaveLength(0);

	for (const notify of observers) {
		notify([{ isIntersecting: true, target: link() }]);
	}

	expect(calls).toHaveLength(1);
});

it("does not use a prefetch rendered with different signal values", async () => {
	const calls = stubFetch(() => snapshot(page("Next", "")));
	const rt = mount(
		`${declaration("a", 1)}<a data-topcoat-link="intent" href="/next">x</a>`,
	);
	link().dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
	await settle();

	rt.context.signal("a").set(new F64(2));
	click(link());
	await settle();

	expect(calls).toHaveLength(2);
	expect(JSON.parse(calls[1]?.init.body as string)).toEqual({
		signals: { a: 2 },
	});
});

it("does not use a prefetch whose response must not be stored", async () => {
	const calls = stubFetch(() =>
		snapshot(page("Next", ""), { "Cache-Control": "private, no-store" }),
	);
	mount(`<a data-topcoat-link="intent" href="/next">x</a>`);
	link().dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
	await settle();

	click(link());
	await settle();

	expect(calls).toHaveLength(2);
	expect(location.pathname).toBe("/next");
});

it("discards prefetches once server state may have changed", async () => {
	const calls = stubFetch(() => snapshot(page("Next", "")));
	mount(`<a data-topcoat-link="intent" href="/next">x</a>`);
	link().dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
	await settle();

	invalidatePrefetches();
	click(link());
	await settle();

	expect(calls).toHaveLength(2);
});

it("starts the destination's connected render once its response has ended", async () => {
	const opened: string[] = [];
	vi.stubGlobal(
		"WebSocket",
		class extends EventTarget {
			readyState = 0;
			constructor(url: string) {
				super();
				opened.push(url);
			}
			send() {}
			close() {}
		},
	);
	const stream = streamed();
	stubFetch(() => stream.response);
	mount(`<a data-topcoat-link="never" href="/live">x</a>`);

	click(link());
	stream.push({
		t: "snapshot",
		html: page("Live", "<!--::topcoat::connect--><p>live</p>"),
	});
	await settle();
	expect(document.body.textContent).toContain("live");
	expect(opened).toHaveLength(0);

	stream.close();
	await settle();

	expect(opened).toHaveLength(1);
	expect(new URL(opened[0] as string).pathname).toBe("/live");
});

it("moves the connected render to the destination when both pages are live", async () => {
	const sockets: { sent: Record<string, unknown>[]; open(): void }[] = [];
	vi.stubGlobal(
		"WebSocket",
		class extends EventTarget {
			readyState = 0;
			readonly sent: Record<string, unknown>[] = [];
			constructor() {
				super();
				sockets.push(this);
			}
			send(data: string) {
				this.sent.push(JSON.parse(data));
			}
			close() {}
			open() {
				this.readyState = 1;
				this.dispatchEvent(new Event("open"));
			}
		},
	);
	stubFetch(() =>
		snapshot(page("Chat", "<!--::topcoat::connect--><p>chat</p>")),
	);
	mount(
		`<!--::topcoat::connect--><a data-topcoat-link="never" href="/chat">x</a>`,
	);
	await settle();
	sockets[0]?.open();

	click(link());
	await settle();

	expect(document.body.textContent).toContain("chat");
	expect(sockets).toHaveLength(1);
	const [first, ...rest] = sockets[0]?.sent ?? [];
	expect(first).toEqual(expect.objectContaining({ run: 1, path: "/" }));
	expect(rest).toEqual([
		{ stop: 1 },
		expect.objectContaining({ run: 2, path: "/chat" }),
	]);
});
