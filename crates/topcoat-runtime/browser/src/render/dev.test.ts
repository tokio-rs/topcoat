// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { PageRefresh } from "../../../../topcoat-cli/browser/src/refresh";
import { DEV_RUNTIME_EVENT, type DevRuntimeDetail } from "../../../../topcoat-core/browser/dev";
import { flushEffects } from "../reactivity";
import { Runtime } from "../runtime";
import { F64 } from "../surrogate";

afterEach(() => {
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
	document.head.innerHTML = "";
	document.body.innerHTML = "";
});

const declaration = (id: string, value: number) =>
	`<!--::topcoat::signal({"t":"signal","id":"${id}","v":${value}})-->`;

it("a dev refresh keeps page and shard signals while replacing bindings and head content", async () => {
	vi.spyOn(document, "readyState", "get").mockReturnValue("complete");
	vi.spyOn(window, "scrollTo").mockImplementation(() => {});
	vi.stubGlobal("location", { href: "http://localhost/search?q=x", pathname: "/search", search: "?q=x", reload: vi.fn() });
	document.head.innerHTML = `<title>Old</title>${declaration("a", 1)}`;
	const shard = `<!--::topcoat::shard::start("1", "0", [])-->${declaration("b", 2)}<p>shard</p><!--::topcoat::shard::end("0")-->`;
	document.body.innerHTML = `${declaration("removed", 1)}<button data-topcoat-on:click="() => cx.signal('a').increment()">add</button><input data-topcoat-bind:value="cx.signal('a').get()">${shard}`;
	const runtime = new Runtime();
	runtime.start(document);
	runtime.page.listenForDevRefresh();
	try {
		const count = runtime.context.signal("a");
		count.set(new F64(5));
		runtime.context.signal("b").set(new F64(3));
		flushEffects();
		const input = document.querySelector("input");
		const button = document.querySelector("button")!;
		const doctype = document.doctype ? "<!doctype html>" : "";
		const html = `${doctype}<html><head><title>New</title>${declaration("a", 0)}</head><body>${declaration("added", 4)}<button data-topcoat-on:click="() => { cx.signal('a').increment(); cx.signal('a').increment(); }">add two</button><input data-topcoat-bind:value="cx.signal('a').get()">${shard}</body></html>`;
		let resolve!: (value: Response) => void;
		const pending = new Promise<Response>((done) => { resolve = done; });
		const fetch = vi.fn().mockReturnValue(pending);
		vi.stubGlobal("fetch", fetch);
		const reportError = vi.fn();
		const refresh = new PageRefresh(() => false, () => {}, reportError);
		const task = refresh.refresh();
		await Promise.resolve();
		expect(fetch.mock.calls[0]?.[0]).toBe("/_topcoat/runtime/pages/search?q=x");
		expect(JSON.parse(fetch.mock.calls[0]?.[1].body)).toEqual({ signals: { a: 5, b: 3, removed: 1 } });
		// A change made while the server is rendering must survive too.
		count.set(new F64(7));
		resolve(new Response(html, { headers: { "Content-Type": "text/html" } }));
		await task;
		flushEffects();

		expect(reportError).not.toHaveBeenCalled();
		expect(location.reload).not.toHaveBeenCalled();
		expect(document.title).toBe("New");
		expect(document.querySelector("input")).toBe(input);
		expect(input?.value).toBe("7");
		expect((runtime.registry.read("b") as F64).dehydrate()).toBe(3);
		expect(runtime.registry.has("removed")).toBe(false);
		expect((runtime.registry.read("added") as F64).dehydrate()).toBe(4);
		expect(document.querySelector("button")).toBe(button);
		button.click();
		flushEffects();
		expect(input?.value).toBe("9");
	} finally {
		runtime.page.dispose();
	}
});

it("disposing the page removes its dev hook", () => {
	const runtime = new Runtime();
	runtime.page.listenForDevRefresh();
	runtime.page.dispose();
	const detail: DevRuntimeDetail = {};
	window.dispatchEvent(new CustomEvent(DEV_RUNTIME_EVENT, { detail }));
	expect(detail.runtime).toBeUndefined();
});
