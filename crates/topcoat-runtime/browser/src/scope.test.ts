// @vitest-environment happy-dom
import { tick } from "@maverick-js/signals";
import { afterEach, beforeEach, expect, it } from "vitest";

import { Runtime } from "./runtime";
import {
	PAGE_ROUTE_PREFIX,
	type Scope,
	SHARD_ROUTE_PREFIX,
	ShardUnit,
	Unit,
} from "./scope";
import type { SignalId } from "./signal";
import { F64 } from "./surrogate";

const originalFetch = globalThis.fetch;
const originalLocation = globalThis.location;

beforeEach(() => {
	document.body.innerHTML = "";
});

afterEach(() => {
	globalThis.fetch = originalFetch;
	globalThis.location = originalLocation;
});

/** Answers every fetch with `status`, recording the last request. */
function stubFetch(status: number, statusText: string, body = "") {
	let url: string | undefined;
	let request: RequestInit | undefined;
	globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
		url = String(input);
		request = init;
		return new Response(body, { status, statusText });
	}) as typeof fetch;
	return { url: () => url, request: () => request };
}

/** Waits for scheduled effects and the fetches they queue to settle. */
async function settle(): Promise<void> {
	tick();
	// A fetch and the replacement it ends in span a number of microtasks
	// that depends on how the platform reads the response body, so yield to
	// a macrotask, which runs only once the microtask queue is empty.
	await new Promise((resolve) => setTimeout(resolve, 0));
	tick();
}

/** Reaches the re-run a unit performs when an input changes. */
function refetch(unit: Unit): () => Promise<void> {
	return (
		unit as unknown as { fetchAndReplace(): Promise<void> }
	).fetchAndReplace.bind(unit);
}

/**
 * Mounts a shard with `content` between marker comments in the body and
 * scans it, the way the runtime does on load.
 */
function mountShard(content: string) {
	document.body.innerHTML = `<p>outside</p><!--::topcoat::shard::start("1", "0", [])-->${content}<!--::topcoat::shard::end("0")-->`;
	const runtime = new Runtime();
	runtime.start(document);
	const [shard] = runtime.page.contentScope.children;
	if (!(shard instanceof ShardUnit)) throw new Error("No shard was scanned");
	return { runtime, shard, fetchAndReplace: refetch(shard) };
}

it("a shard sends the identity in a header and the arguments and signal values in the body", async () => {
	const stub = stubFetch(500, "Internal Server Error");
	const { fetchAndReplace, runtime, shard } = mountShard("");
	runtime.registry.insert("s1", new F64(3));
	shard.contentScope.signalIds.add("s1");

	await fetchAndReplace().catch(() => undefined);

	expect(stub.url()).toBe(`${SHARD_ROUTE_PREFIX}/1`);
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

it("a page posts the values of every signal in the document to the pages route", async () => {
	const stub = stubFetch(500, "Internal Server Error");
	globalThis.location = {
		pathname: "/search",
		search: "?q=shoes",
	} as unknown as Location;

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

	expect(stub.url()).toBe(`${PAGE_ROUTE_PREFIX}/search?q=shoes`);
	expect(JSON.parse(stub.request()?.body as string)).toEqual({
		signals: { p1: 1, s1: 2 },
	});
	expect(String(error)).toContain("Page request failed");
});

it("the root page posts to the bare pages route", async () => {
	const stub = stubFetch(500, "Internal Server Error");
	globalThis.location = { pathname: "/", search: "" } as unknown as Location;

	const runtime = new Runtime();
	await refetch(runtime.page)().catch(() => undefined);

	expect(stub.url()).toBe(PAGE_ROUTE_PREFIX);
});

it("a page re-run morphs the body, keeping a focused input", async () => {
	stubFetch(
		200,
		"OK",
		`<!doctype html><html><head><title>t</title></head><body><input><p>2 results</p></body></html>`,
	);
	globalThis.location = { pathname: "/", search: "" } as unknown as Location;
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
class ScriptedUnit extends Unit {
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
