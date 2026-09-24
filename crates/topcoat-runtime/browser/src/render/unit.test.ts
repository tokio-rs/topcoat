// @vitest-environment happy-dom
import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { flushEffects } from "../reactivity";
import { Runtime } from "../runtime";
import type { Scope } from "../scope";
import type { SignalId } from "../signal-registry";
import { F64 } from "../surrogate";
import { RUNTIME_PROTOCOL } from "./connection";
import { RUNTIME_HEADER } from "./request";
import { ShardUnit } from "./shard";
import { RenderUnit } from "./unit";

const originalFetch = globalThis.fetch;
const originalLocation = globalThis.location;

beforeEach(() => {
	document.body.innerHTML = "";
});

afterEach(() => {
	globalThis.fetch = originalFetch;
	globalThis.location = originalLocation;
	vi.unstubAllGlobals();
	vi.restoreAllMocks();
});

/** Answers every fetch with `status`, recording the last request. */
function stubFetch(status: number, statusText: string, body = "") {
	let url: string | undefined;
	let request: RequestInit | undefined;
	let calls = 0;
	globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
		url = String(input);
		request = init;
		calls += 1;
		return new Response(body, { status, statusText });
	}) as typeof fetch;
	return { url: () => url, request: () => request, calls: () => calls };
}

/** Waits for scheduled effects and the fetches they queue to settle. */
async function settle(): Promise<void> {
	flushEffects();
	// A fetch and the replacement it ends in span a number of microtasks
	// that depends on how the platform reads the response body, so yield to
	// a macrotask, which runs only once the microtask queue is empty.
	await new Promise((resolve) => setTimeout(resolve, 0));
	flushEffects();
}

/** Reaches the re-run a unit performs when an input changes. */
function refetch(unit: RenderUnit): () => Promise<void> {
	return () => unit.refresh();
}

/**
 * Mounts a shard with `content` between marker comments in the body and
 * hydrates its content and starts watching its inputs.
 */
function mountShard(content: string) {
	document.body.innerHTML = `<p>outside</p><!--::topcoat::shard::start("/shards/1", "0", [])-->${content}<!--::topcoat::shard::end("0")-->`;
	const runtime = new Runtime();
	const start = document.body.childNodes[1] as Comment;
	const end = document.body.lastChild as Comment;
	const shard = new ShardUnit(
		runtime.page.contentScope,
		runtime,
		"/shards/1",
		"0",
		[],
		start,
	);
	shard.attachEnd(end);
	runtime.hydrate(document.body, start, end, shard.contentScope);
	shard.startWatching();
	return { runtime, shard, fetchAndReplace: refetch(shard) };
}

it("a shard sends the identity in a header and the arguments and signal values in the body", async () => {
	const stub = stubFetch(500, "Internal Server Error");
	const { fetchAndReplace, runtime, shard } = mountShard("");
	runtime.registry.insert("s1", new F64(3));
	shard.contentScope.signalIds.add("s1");

	await fetchAndReplace().catch(() => undefined);

	expect(stub.url()).toBe("/shards/1");
	const headers = stub.request()?.headers as Record<string, string>;
	expect(headers["X-Topcoat-Identity"]).toBe("0");
	expect(JSON.parse(stub.request()?.body as string)).toEqual({
		args: [],
		signals: { s1: 3 },
	});
});

it("a shard keeps the rendered content when the server responds with an error", async () => {
	stubFetch(405, "Method Not Allowed");
	const { fetchAndReplace } = mountShard("<p>rendered</p>");
	const before = document.body.innerHTML;

	const error = await fetchAndReplace().then(
		() => undefined,
		(e: unknown) => e,
	);

	expect(document.body.innerHTML).toBe(before);
	expect(String(error)).toContain(
		"Shard request failed: 405 Method Not Allowed",
	);
});

it("a shard re-run morphs its content, keeping a focused input and the markers", async () => {
	stubFetch(200, "OK", `<input value="shoes"><ul><li>shoes</li></ul>`);
	const { fetchAndReplace } = mountShard(
		`<input value="sho"><ul><li>shoe</li><li>shorts</li></ul>`,
	);
	const input = document.querySelector("input") as HTMLInputElement;
	input.focus();
	input.value = "shoes";
	const markers = Array.from(document.body.childNodes).filter(
		(node) => node.nodeType === Node.COMMENT_NODE,
	);

	await fetchAndReplace();

	expect(document.querySelector("input")).toBe(input);
	expect(document.activeElement).toBe(input);
	expect(input.value).toBe("shoes");
	expect(
		Array.from(document.body.childNodes).filter(
			(node) => node.nodeType === Node.COMMENT_NODE,
		),
	).toEqual(markers);
	expect(document.querySelector("ul")?.innerHTML).toBe(`<li>shoes</li>`);
});

it("a re-run attaches each event handler once", async () => {
	const clicks = "globalThis.clicks = (globalThis.clicks ?? 0) + 1";
	const button = `<button data-topcoat-on:click="() => { ${clicks}; }">go</button>`;
	stubFetch(200, "OK", button);
	const { fetchAndReplace } = mountShard(button);
	const el = document.querySelector("button") as HTMLButtonElement;

	await fetchAndReplace();
	el.click();

	expect(document.querySelector("button")).toBe(el);
	expect((globalThis as { clicks?: number }).clicks).toBe(1);
});

it("a page posts the values of every signal in the document to its own URL", async () => {
	const stub = stubFetch(500, "Internal Server Error");
	globalThis.location = new URL(
		"https://app.example/search?q=shoes",
	) as unknown as Location;

	const runtime = new Runtime();
	runtime.registry.insert("p1", new F64(1));
	runtime.page.contentScope.signalIds.add("p1");
	// A signal a nested shard's content owns travels with the page as well.
	const shard = new ShardUnit(
		runtime.page.contentScope,
		runtime,
		"1",
		"0",
		[],
		{} as Comment,
	);
	runtime.registry.insert("s1", new F64(2));
	shard.contentScope.signalIds.add("s1");

	const error = await refetch(runtime.page)().then(
		() => undefined,
		(e: unknown) => e,
	);

	expect(stub.url()).toBe("https://app.example/search?q=shoes");
	expect(stub.request()?.method).toBe("POST");
	const headers = stub.request()?.headers as Record<string, string>;
	expect(headers[RUNTIME_HEADER]).toBe("true");
	expect(JSON.parse(stub.request()?.body as string)).toEqual({
		signals: { p1: 1, s1: 2 },
	});
	expect(String(error)).toContain("Page request failed");
});

it("the root page posts to its own URL", async () => {
	const stub = stubFetch(500, "Internal Server Error");
	globalThis.location = new URL("https://app.example/") as unknown as Location;

	const runtime = new Runtime();
	await refetch(runtime.page)().catch(() => undefined);

	expect(stub.url()).toBe("https://app.example/");
});

it("a page with a double-slash path posts signals to its own origin", async () => {
	const stub = stubFetch(500, "Internal Server Error");
	globalThis.location = new URL(
		"https://app.example:8443//other.example/path?q=shoes#section",
	) as unknown as Location;

	const runtime = new Runtime();
	await refetch(runtime.page)().catch(() => undefined);

	const requestUrl = new URL(stub.url() as string, location.href);
	expect(requestUrl.origin).toBe(location.origin);
	expect(requestUrl.pathname).toBe(location.pathname);
	expect(requestUrl.search).toBe(location.search);
	expect(requestUrl.hash).toBe("");
});

it("a page re-run morphs the body, keeping a focused input", async () => {
	stubFetch(
		200,
		"OK",
		`<!doctype html><html><head><title>t</title></head><body><input><p>2 results</p></body></html>`,
	);
	globalThis.location = new URL("https://app.example/") as unknown as Location;
	document.body.innerHTML = `<input><p>1 result</p>`;
	const runtime = new Runtime();
	runtime.start(document);
	const input = document.querySelector("input") as HTMLInputElement;
	input.focus();

	await refetch(runtime.page)();

	expect(document.querySelector("input")).toBe(input);
	expect(document.activeElement).toBe(input);
	expect(document.body.innerHTML).toBe(`<input><p>2 results</p>`);
});

/**
 * A unit whose content is described by a script rather than a document:
 * each replacement declares the given signals and dependencies into the new
 * content scope, the way a scan of real markup would.
 */
class ScriptedUnit extends RenderUnit {
	protected readonly label = "Scripted";
	readonly requests: number[] = [];
	/** The content each successive replacement scans. */
	readonly script: { signals: [SignalId, unknown][]; deps: SignalId[] }[] = [];

	constructor(runtime: Runtime) {
		super(null, runtime);
	}

	protected readInputs(): void {}

	protected request(): Promise<Response> {
		this.requests.push(this.requests.length);
		return Promise.resolve(new Response("", { status: 200 }));
	}

	protected prepare(): Node[] | null {
		return [];
	}

	protected insert(
		_nodes: Node[],
		scope: Scope,
		adoptable: Set<SignalId>,
	): void {
		const content = this.script.shift();
		if (!content) throw new Error("No content scripted");
		const { registry } = this.runtime;
		for (const [id, value] of content.signals) {
			if (registry.insert(id, value) || adoptable.delete(id)) {
				scope.signalIds.add(id);
			}
		}
		for (const id of content.deps) scope.dependencies.add(id);
	}
}

/** Mounts a scripted unit whose first content is the first script entry. */
function mountScripted() {
	const runtime = new Runtime();
	const unit = new ScriptedUnit(runtime);
	return {
		runtime,
		unit,
		/** Makes `content` the current content, as a replacement would. */
		render(content: (typeof unit.script)[number]) {
			unit.script.push(content);
			unit.replaceContent("");
		},
	};
}

it("a change to a dependency re-fetches the unit once", async () => {
	const { runtime, unit, render } = mountScripted();
	render({ signals: [["a", new F64(0)]], deps: ["a"] });
	expect(unit.requests).toEqual([]);

	const a = runtime.context.signal("a");
	a.set(new F64(1));
	a.set(new F64(2));
	unit.script.push({ signals: [["a", new F64(0)]], deps: ["a"] });
	await settle();

	expect(unit.requests).toEqual([0]);
});

it("the watch effect follows the dependencies of the new content", async () => {
	const { runtime, unit, render } = mountScripted();
	render({
		signals: [
			["a", new F64(0)],
			["b", new F64(0)],
		],
		deps: ["a"],
	});

	// The replacement depends on `b` instead of `a`.
	unit.script.push({
		signals: [
			["a", new F64(0)],
			["b", new F64(0)],
		],
		deps: ["b"],
	});
	runtime.context.signal("a").set(new F64(1));
	await settle();
	expect(unit.requests).toEqual([0]);

	runtime.context.signal("a").set(new F64(2));
	await settle();
	expect(unit.requests).toEqual([0]);

	unit.script.push({ signals: [], deps: [] });
	runtime.context.signal("b").set(new F64(1));
	await settle();
	expect(unit.requests).toEqual([0, 1]);
});

it("a replacement keeps the signals the new content declares again and deletes the rest", () => {
	const { runtime, unit, render } = mountScripted();
	render({
		signals: [
			["kept", new F64(0)],
			["dropped", new F64(0)],
		],
		deps: [],
	});
	runtime.context.signal("kept").set(new F64(5));

	render({ signals: [["kept", new F64(0)]], deps: [] });

	// The existing signal wins over the declaration, so the client's value
	// survives, and the new content owns it.
	expect((runtime.registry.read("kept") as F64).dehydrate()).toBe(5);
	expect(unit.contentScope.signalIds).toEqual(new Set(["kept"]));
	expect(runtime.registry.has("dropped")).toBe(false);
});

it("disposing a unit deletes its signals", () => {
	const { runtime, unit, render } = mountScripted();
	render({ signals: [["a", new F64(0)]], deps: [] });

	unit.dispose();

	expect(runtime.registry.has("a")).toBe(false);
	expect(unit.isDisposed).toBe(true);
	expect(unit.contentScope.isDisposed).toBe(true);
});

it("a connection requirement anywhere in the page content belongs to the page", () => {
	document.body.innerHTML = `<div><p>text</p><!--::topcoat::connect--></div>`;
	const runtime = new Runtime();
	try {
		runtime.start(document);
		expect(runtime.page.requiresConnection).toBe(true);
	} finally {
		runtime.page.dispose();
	}
});

it("a page without a connection requirement does not require one", () => {
	document.body.innerHTML = `<div><!--::topcoat::signal({"t":"signal","id":"a","v":1})--><!--::topcoat::dep("a")--><p>text</p></div>`;
	const runtime = new Runtime();
	try {
		runtime.start(document);
		expect(runtime.page.requiresConnection).toBe(false);
	} finally {
		runtime.page.dispose();
	}
});

it("a connection requirement inside a shard belongs to the shard, not the page", () => {
	const { runtime, shard } = mountShard(`<p>text</p><!--::topcoat::connect-->`);

	expect(shard.requiresConnection).toBe(true);
	expect(runtime.page.requiresConnection).toBe(false);
});

const declaration = (id: string, value: number) =>
	`<!--::topcoat::signal({"t":"signal","id":"${id}","v":${value}})-->`;

/** Mounts a page whose body holds `outside` and the live region `aa`. */
function mountRegion(outside: string, region: string) {
	document.body.innerHTML = `${outside}<!--::topcoat::region::start(aa)-->${region}<!--::topcoat::region::end(aa)-->`;
	const runtime = new Runtime();
	runtime.start(document);
	return runtime;
}

it("a swap replaces its region's content, rebuilding the region's resources and keeping its surviving signals", async () => {
	const button = `<button data-topcoat-on:click="() => cx.signal('a').increment()">go</button>`;
	const runtime = mountRegion(
		`<p>outside</p>`,
		`${declaration("a", 0)}${declaration("b", 0)}${button}<p>one</p>`,
	);
	try {
		runtime.context.signal("a").set(new F64(5));
		const el = document.querySelector("button") as HTMLButtonElement;
		const before = runtime.page.contentScope.regions.get("aa")?.scope;

		runtime.page.applySwap("aa", `${declaration("a", 0)}${button}<p>two</p>`);

		expect(document.body.innerHTML).toBe(
			`<p>outside</p><!--::topcoat::region::start(aa)-->${declaration("a", 0)}${button}<p>two</p><!--::topcoat::region::end(aa)-->`,
		);
		// The client's value survives, and the region owns it again.
		expect((runtime.registry.read("a") as F64).dehydrate()).toBe(5);
		expect(runtime.registry.has("b")).toBe(false);
		const region = runtime.page.contentScope.regions.get("aa");
		expect(region?.scope).not.toBe(before);
		expect(before?.isDisposed).toBe(true);
		expect(region?.scope.signalIds).toEqual(new Set(["a"]));
		expect(runtime.page.contentScope.signalIds).toEqual(new Set());

		// The morph kept the button; only the new content's handler is attached.
		expect(document.querySelector("button")).toBe(el);
		el.click();
		await Promise.resolve();
		expect((runtime.registry.read("a") as F64).dehydrate()).toBe(6);
	} finally {
		runtime.page.dispose();
	}
});

it("a swap for a region the content does not have is ignored", () => {
	const runtime = mountRegion(`<p>outside</p>`, `<p>one</p>`);
	try {
		const before = document.body.innerHTML;

		runtime.page.applySwap("zz", `<p>two</p>`);

		expect(document.body.innerHTML).toBe(before);
	} finally {
		runtime.page.dispose();
	}
});

it("a dependency a swap declares re-renders the unit when it changes", async () => {
	const stub = stubFetch(500, "Internal Server Error");
	globalThis.location = new URL("https://app.example/") as unknown as Location;
	const runtime = mountRegion(``, `<p>one</p>`);
	try {
		runtime.page.applySwap(
			"aa",
			`${declaration("c", 0)}<!--::topcoat::dep("c")-->`,
		);
		expect(runtime.page.contentScope.collectDependencies()).toEqual(
			new Set(["c"]),
		);

		runtime.context.signal("c").set(new F64(1));
		await settle();

		expect(stub.url()).toBe("https://app.example/");
	} finally {
		runtime.page.dispose();
	}
});

it("a swap parses its content in the region's context, so table rows survive", () => {
	document.body.innerHTML = `<table><tbody><!--::topcoat::region::start(aa)--><tr><td>one</td></tr><!--::topcoat::region::end(aa)--></tbody></table>`;
	const runtime = new Runtime();
	runtime.start(document);
	try {
		runtime.page.applySwap("aa", `<tr><td>two</td></tr>`);

		expect(document.querySelectorAll("tr")).toHaveLength(1);
		expect(document.querySelector("tbody")?.innerHTML).toBe(
			`<!--::topcoat::region::start(aa)--><tr><td>two</td></tr><!--::topcoat::region::end(aa)-->`,
		);
	} finally {
		runtime.page.dispose();
	}
});

it("a shard re-run parses its content in the shard's context, so table rows survive", async () => {
	stubFetch(200, "OK", `<tr><td>two</td></tr>`);
	document.body.innerHTML = `<table><tbody><!--::topcoat::shard::start("/shards/1", "0", [])--><tr><td>one</td></tr><!--::topcoat::shard::end("0")--></tbody></table>`;
	const runtime = new Runtime();
	runtime.start(document);
	try {
		const shard = runtime.page.contentScope.children.values().next().value
			?.unit as ShardUnit;
		await shard.refresh();

		expect(document.querySelectorAll("tr")).toHaveLength(1);
		expect(document.querySelector("td")?.textContent).toBe("two");
	} finally {
		runtime.page.dispose();
	}
});

it("a swap that removes a signal drops the dependency on it", () => {
	const runtime = mountRegion(
		``,
		`${declaration("c", 0)}<!--::topcoat::dep("c")-->`,
	);
	try {
		expect(runtime.page.contentScope.collectDependencies()).toEqual(
			new Set(["c"]),
		);

		runtime.page.applySwap("aa", `<p>gone</p>`);

		expect(runtime.registry.has("c")).toBe(false);
		expect(runtime.page.contentScope.collectDependencies()).toEqual(new Set());
	} finally {
		runtime.page.dispose();
	}
});

it("a swap inside a shard re-subscribes the shard, not the page", async () => {
	const stub = stubFetch(500, "Internal Server Error");
	globalThis.location = new URL("https://app.example/") as unknown as Location;
	const { runtime, shard } = mountShard(
		`<!--::topcoat::region::start(aa)--><p>one</p><!--::topcoat::region::end(aa)-->`,
	);
	try {
		// The page receives the frame and finds the region inside the shard.
		runtime.page.applySwap(
			"aa",
			`${declaration("d", 0)}<!--::topcoat::dep("d")-->`,
		);
		expect(shard.contentScope.collectDependencies()).toEqual(new Set(["d"]));
		expect(runtime.page.contentScope.collectDependencies()).toEqual(new Set());

		runtime.context.signal("d").set(new F64(1));
		await settle();

		expect(stub.calls()).toBe(1);
		expect(stub.url()).toBe("/shards/1");
	} finally {
		runtime.page.dispose();
	}
});

/** A WebSocket the test drives by hand, installed as the global. */
class FakeSocket extends EventTarget {
	static opened: FakeSocket[] = [];
	readyState = 0;
	readonly sent: { run: number; signals: Record<string, unknown> }[] = [];

	constructor(
		readonly url: string,
		readonly protocol: string,
	) {
		super();
		FakeSocket.opened.push(this);
	}

	send(data: string): void {
		this.sent.push(JSON.parse(data));
	}

	close(): void {
		this.readyState = 3;
		this.dispatchEvent(new Event("close"));
	}

	open(): void {
		this.readyState = 1;
		this.dispatchEvent(new Event("open"));
	}

	receive(message: unknown): void {
		this.dispatchEvent(
			new MessageEvent("message", { data: JSON.stringify(message) }),
		);
	}
}

/** Installs the fake socket and a page URL, with the document loaded. */
function installSocket(readyState = "complete") {
	FakeSocket.opened = [];
	vi.stubGlobal("WebSocket", FakeSocket);
	globalThis.location = new URL(
		"https://app.example/room?q=1",
	) as unknown as Location;
	vi.spyOn(document, "readyState", "get").mockReturnValue(
		readyState as DocumentReadyState,
	);
}

it("a page whose content asks for a connection opens one at its own URL once the document has loaded", async () => {
	installSocket("loading");
	const stub = stubFetch(500, "Internal Server Error");
	document.body.innerHTML = `${declaration("a", 1)}<!--::topcoat::dep("a")--><!--::topcoat::connect--><p>1</p>`;
	const runtime = new Runtime();
	runtime.start(document);
	try {
		expect(FakeSocket.opened).toHaveLength(0);
		window.dispatchEvent(new Event("load"));
		expect(FakeSocket.opened).toHaveLength(1);
		const socket = FakeSocket.opened[0] as FakeSocket;
		expect(socket.url).toBe("wss://app.example/room?q=1");
		expect(socket.protocol).toBe(RUNTIME_PROTOCOL);

		socket.open();
		expect(socket.sent).toEqual([{ run: 1, signals: { a: 1 } }]);

		// A re-render over the open connection is a new run, not a post.
		runtime.context.signal("a").set(new F64(2));
		await settle();
		expect(socket.sent).toEqual([
			{ run: 1, signals: { a: 1 } },
			{ run: 2, signals: { a: 2 } },
		]);
		expect(stub.url()).toBe(undefined);

		// The run's snapshot morphs the body and keeps the connection.
		socket.receive({ t: "run", id: 2 });
		socket.receive({
			t: "snapshot",
			html: `<!doctype html><html><body>${declaration("a", 2)}<!--::topcoat::dep("a")--><!--::topcoat::connect--><p>2</p></body></html>`,
		});
		expect(document.querySelector("p")?.textContent).toBe("2");
		expect(FakeSocket.opened).toHaveLength(1);
		runtime.context.signal("a").set(new F64(3));
		await settle();
		expect(socket.sent).toHaveLength(3);
	} finally {
		runtime.page.dispose();
	}
});

it("a page whose content does not ask for a connection opens none", () => {
	installSocket();
	document.body.innerHTML = `${declaration("a", 1)}<p>1</p>`;
	const runtime = new Runtime();
	runtime.start(document);
	try {
		expect(FakeSocket.opened).toHaveLength(0);
	} finally {
		runtime.page.dispose();
	}
});

it("while the connection is not open, a re-render posts over HTTP", async () => {
	installSocket();
	const stub = stubFetch(500, "Internal Server Error");
	document.body.innerHTML = `${declaration("a", 1)}<!--::topcoat::dep("a")--><!--::topcoat::connect-->`;
	const runtime = new Runtime();
	runtime.start(document);
	try {
		const socket = FakeSocket.opened[0] as FakeSocket;
		socket.open();
		socket.close();

		runtime.context.signal("a").set(new F64(2));
		await settle();

		expect(stub.url()).toBe("https://app.example/room?q=1");
		expect(socket.sent).toHaveLength(1);
	} finally {
		runtime.page.dispose();
	}
});

it("a re-run re-evaluates the connection requirement from the new content", async () => {
	stubFetch(200, "OK", `<p>quiet</p>`);
	const { fetchAndReplace, shard } = mountShard(
		`<p>text</p><!--::topcoat::connect-->`,
	);
	expect(shard.requiresConnection).toBe(true);

	await fetchAndReplace();
	expect(shard.requiresConnection).toBe(false);

	stubFetch(200, "OK", `<p>live</p><!--::topcoat::connect-->`);
	await fetchAndReplace();
	expect(shard.requiresConnection).toBe(true);
});
