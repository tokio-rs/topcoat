import { afterEach, expect, it } from "vitest";

import { Runtime } from "./runtime";
import { ReactiveScope } from "./scope";

const originalFetch = globalThis.fetch;

afterEach(() => {
	globalThis.fetch = originalFetch;
});

function mount(status: number, statusText: string) {
	globalThis.fetch = (async () =>
		new Response("", { status, statusText })) as typeof fetch;

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
		[],
		start,
	);
	scope.attachEnd(end);

	const fetchAndReplace = (
		scope as unknown as { fetchAndReplace(): Promise<void> }
	).fetchAndReplace.bind(scope);

	return { fetchAndReplace, removed };
}

it("keeps the rendered content when the shard responds with an error", async () => {
	const { fetchAndReplace, removed } = mount(405, "Method Not Allowed");

	const error = await fetchAndReplace().then(
		() => undefined,
		(e: unknown) => e,
	);

	expect(removed).toEqual([]);
	expect(String(error)).toContain("405 Method Not Allowed");
});
