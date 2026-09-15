// @vitest-environment happy-dom
import { tick } from "@maverick-js/signals";
import { afterEach, expect, it, vi } from "vitest";
import { Runtime } from "../runtime";
import { F64, WriteSignal } from "../surrogate";
import { parseComment } from "./markers";

afterEach(() => {
	vi.unstubAllGlobals();
	document.body.innerHTML = "";
});

it("parses signal values without resolving their references", () => {
	const marker = document.createComment(
		'::topcoat::signal({"t":"signal","id":"b","v":{"t":"Signal","id":"a"}})',
	);

	expect(parseComment(marker)).toEqual({
		kind: "signal",
		id: "b",
		value: { t: "Signal", id: "a" },
	});
});

it("hydrates signal references using the runtime registry", () => {
	const root = document.createElement("div");
	root.innerHTML = `
		<!--::topcoat::signal({"t":"signal","id":"a","v":1})-->
		<!--::topcoat::signal({"t":"signal","id":"b","v":{"t":"Signal","id":"a"}})-->
	`;
	const runtime = new Runtime();
	try {
		runtime.start(root);
		const reference = runtime.registry.read("b");
		expect(reference).toBeInstanceOf(WriteSignal);
		if (!(reference instanceof WriteSignal)) throw new Error("Missing signal");

		runtime.context.signal("a").set(new F64(2));
		expect(reference.get().dehydrate()).toBe(2);
	} finally {
		runtime.page.dispose();
	}
});

it("a page replacement releases nested shards and adopts surviving signals", async () => {
	document.body.innerHTML = `
		<!--::topcoat::signal({"t":"signal","id":"a","v":1})-->
		<!--::topcoat::shard::start("1", "0", [])-->
			<!--::topcoat::shard::start("2", "1", [])-->
				<!--::topcoat::signal({"t":"signal","id":"b","v":0})-->
				<!--::topcoat::dep("b")-->
			<!--::topcoat::shard::end("1")-->
		<!--::topcoat::shard::end("0")-->
	`;
	vi.stubGlobal("location", { pathname: "/", search: "" });
	let finishShard!: (response: Response) => void;
	const pendingShard = new Promise<Response>((resolve) => {
		finishShard = resolve;
	});
	const fetch = vi.fn((url: string, _init: RequestInit) =>
		url.includes("/shards/")
			? pendingShard
			: Promise.resolve(
					new Response(`
						<body>
							<!--::topcoat::signal({"t":"signal","id":"a","v":99})-->
							<p>replacement</p>
						</body>
					`),
				),
	);
	vi.stubGlobal("fetch", fetch);
	const runtime = new Runtime();
	try {
		runtime.start(document);
		runtime.context.signal("a").set(new F64(7));
		runtime.context.signal("b").set(new F64(1));
		tick();
		await Promise.resolve();

		expect(fetch).toHaveBeenCalledTimes(1);
		expect(fetch.mock.calls[0]?.[0]).toContain("/shards/2");
		const childRequest = fetch.mock.calls[0]?.[1].signal;
		await runtime.page.refresh();

		expect(childRequest?.aborted).toBe(true);
		expect(runtime.registry.has("b")).toBe(false);
		expect((runtime.registry.read("a") as F64).dehydrate()).toBe(7);
		expect(runtime.page.contentScope.children.size).toBe(0);

		finishShard(new Response("<p>stale shard</p>"));
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(document.querySelector("p")?.textContent).toBe("replacement");
	} finally {
		runtime.page.dispose();
	}
});
