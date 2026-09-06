import { afterEach, expect, it } from "vitest";

import { Runtime } from "./runtime";
import { ReactiveScope } from "./scope";
import { F64 } from "./surrogate";

const originalFetch = globalThis.fetch;

afterEach(() => {
	globalThis.fetch = originalFetch;
});

function mount(status: number, statusText: string) {
	let request: RequestInit | undefined;
	globalThis.fetch = (async (_input: RequestInfo | URL, init?: RequestInit) => {
		request = init;
		return new Response("", { status, statusText });
	}) as typeof fetch;

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
	const scope = new ReactiveScope(
		runtime.rootScope,
		runtime,
		"scope",
		"/_topcoat/shards/1",
		"0",
		[],
		start,
	);
	scope.attachEnd(end);

	const fetchAndReplace = (
		scope as unknown as { fetchAndReplace(): Promise<void> }
	).fetchAndReplace.bind(scope);

	return {
		fetchAndReplace,
		removed,
		runtime,
		scope,
		request: () => request,
	};
}

it("sends the identity in a header and the arguments and signal values in the body", async () => {
	const { fetchAndReplace, runtime, scope, request } = mount(
		500,
		"Internal Server Error",
	);
	runtime.registry.insert("s1", new F64(3));
	scope.contentScope.signalIds.add("s1");

	await fetchAndReplace().catch(() => undefined);

	const headers = request()?.headers as Record<string, string>;
	expect(headers["X-Topcoat-Identity"]).toBe("0");
	expect(JSON.parse(request()?.body as string)).toEqual({
		args: [],
		signals: { s1: 3 },
	});
});

it("keeps the rendered content when the shard responds with an error", async () => {
	const { fetchAndReplace, removed } = mount(405, "Method Not Allowed");

	const error = await fetchAndReplace().then(
		() => undefined,
		(e: unknown) => e,
	);

	expect(removed).toEqual([]);
	expect(String(error)).toContain("405 Method Not Allowed");
});
