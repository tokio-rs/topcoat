import { tick } from "@maverick-js/signals";
import { afterEach, expect, it } from "vitest";

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

/**
 * Mounts a shard between fake marker nodes, so the DOM removal the
 * replacement performs can be observed without a document.
 */
function mountShard(stub: ReturnType<typeof stubFetch>) {
	const removed: ChildNode[] = [];
	const end = {} as Comment;
	const content = { nextSibling: end } as unknown as ChildNode;
	const parent = {
		removeChild(node: ChildNode) {
			removed.push(node);
			return node;
		},
	} as unknown as ParentNode;
	const start = {
		parentNode: parent,
		nextSibling: content,
	} as unknown as Comment;

	const runtime = new Runtime();
	const shard = new ShardUnit(
		runtime.page.contentScope,
		runtime,
		"scope",
		"1",
		"0",
		[],
		start,
	);
	shard.attachEnd(end);

	const fetchAndReplace = (
		shard as unknown as { fetchAndReplace(): Promise<void> }
	).fetchAndReplace.bind(shard);

	return { fetchAndReplace, removed, runtime, shard, ...stub };
}

it("a shard sends the identity in a header and the arguments and signal values in the body", async () => {
	const { fetchAndReplace, runtime, shard, url, request } = mountShard(
		stubFetch(500, "Internal Server Error"),
	);
	runtime.registry.insert("s1", new F64(3));
	shard.contentScope.signalIds.add("s1");

	await fetchAndReplace().catch(() => undefined);

	expect(url()).toBe(`${SHARD_ROUTE_PREFIX}/1`);
	const headers = request()?.headers as Record<string, string>;
	expect(headers["X-Topcoat-Identity"]).toBe("0");
	expect(JSON.parse(request()?.body as string)).toEqual({
		args: [],
		signals: { s1: 3 },
	});
});

it("a shard keeps the rendered content when the server responds with an error", async () => {
	const { fetchAndReplace, removed } = mountShard(
		stubFetch(405, "Method Not Allowed"),
	);

	const error = await fetchAndReplace().then(
		() => undefined,
		(e: unknown) => e,
	);

	expect(removed).toEqual([]);
	expect(String(error)).toContain(
		"Shard request failed: 405 Method Not Allowed",
	);
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
		"scope",
		"1",
		"0",
		[],
		{} as Comment,
	);
	runtime.registry.insert("s1", new F64(2));
	shard.contentScope.signalIds.add("s1");

	const fetchAndReplace = (
		runtime.page as unknown as { fetchAndReplace(): Promise<void> }
	).fetchAndReplace.bind(runtime.page);
	const error = await fetchAndReplace().then(
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
	const fetchAndReplace = (
		runtime.page as unknown as { fetchAndReplace(): Promise<void> }
	).fetchAndReplace.bind(runtime.page);
	await fetchAndReplace().catch(() => undefined);

	expect(stub.url()).toBe(PAGE_ROUTE_PREFIX);
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

	protected prepare(): DocumentFragment | null {
		return {} as DocumentFragment;
	}

	protected insert(
		_fragment: DocumentFragment,
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
