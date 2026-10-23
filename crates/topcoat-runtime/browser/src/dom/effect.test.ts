// @vitest-environment happy-dom

import { afterEach, expect, it, vi } from "vitest";
import {
	DEV_RUNTIME_EVENT,
	type DevRuntimeDetail,
} from "../../../../topcoat-core/browser/dev";
import { morph } from "../../../../topcoat-core/browser/morph";
import { flushEffects, signal } from "../reactivity";
import { newRender } from "../render/frames";
import { Runtime } from "../runtime";
import type { EffectContext } from "./effect";

const runtimes: Runtime[] = [];

function mount(html: string, hook: (context: EffectContext) => void): Runtime {
	document.body.innerHTML = html;
	vi.stubGlobal("hook", hook);
	const runtime = new Runtime();
	runtimes.push(runtime);
	runtime.start(document);
	flushEffects();
	return runtime;
}

const element = '<button id="owned" data-topcoat-effect="hook">Action</button>';

afterEach(() => {
	for (const runtime of runtimes) runtime.page.dispose();
	runtimes.length = 0;
	flushEffects();
	vi.unstubAllGlobals();
	document.body.innerHTML = "";
});

it("starts after descendant bindings and later signal declarations are hydrated", () => {
	const observed: { connected: boolean; value: string | undefined }[] = [];
	const hook = vi.fn((context: EffectContext) => {
		observed.push({
			connected: context.element.isConnected,
			value: context.element.querySelector("input")?.value,
		});
	});
	mount(
		`<section data-topcoat-effect="hook">
			<!--::topcoat::signal({"t":"signal","id":"a","v":"ready"})-->
			<input data-topcoat-bind:value="cx.signal('a').get()">
		</section>`,
		hook,
	);
	expect(hook).toHaveBeenCalledTimes(1);
	expect(observed).toEqual([{ connected: true, value: "ready" }]);
});

it("cancels initial work when its content is replaced before the queue runs", () => {
	const hook = vi.fn();
	vi.stubGlobal("hook", hook);
	document.body.innerHTML = element;
	const runtime = new Runtime();
	runtimes.push(runtime);
	runtime.start(document);
	runtime.page.replaceContent("<body>Gone</body>", newRender());
	flushEffects();
	expect(hook).not.toHaveBeenCalled();
});

it("subscribes server watchers before an initial effect writes a signal", async () => {
	const fetch = vi.fn(() => new Promise<Response>(() => {}));
	vi.stubGlobal("fetch", fetch);
	mount(
		`<!--::topcoat::signal({"t":"signal","id":"a","v":1})-->
		<!--::topcoat::dep("a")-->
		<input data-topcoat-effect="scope => cx.signal('a').set(cx.hydrate(2))">`,
		() => {},
	);
	await Promise.resolve();
	expect(fetch).toHaveBeenCalledTimes(1);
});

it("tracks conditional reads, cleans before reruns, and cancels queued effects on removal", () => {
	const choose = signal(true);
	const first = signal(1);
	const second = signal(10);
	const log: string[] = [];
	const runtime = mount(element, (context) => {
		const value = choose() ? first() : second();
		log.push(`run ${value}`);
		context.on_cleanup(() => {
			expect(context.is_active().dehydrate()).toBe(false);
			expect(context.abortSignal.aborted).toBe(true);
			log.push(`clean ${value}`);
		});
	});
	first.set(2);
	first.set(3);
	flushEffects();
	choose.set(false);
	flushEffects();
	first.set(4);
	flushEffects();
	second.set(11);
	runtime.page.replaceContent("<body>Removed</body>", newRender());
	flushEffects();
	expect(log).toEqual([
		"run 1",
		"clean 1",
		"run 3",
		"clean 3",
		"run 10",
		"clean 10",
	]);
});

it("cleanup reads do not subscribe and cleanup errors do not strand resources", () => {
	const value = signal(0);
	const cleanupOnly = signal(0);
	const cleaned: number[] = [];
	const error = new Error("cleanup failed");
	const contexts: EffectContext[] = [];
	const runtime = mount(element, (context) => {
		value();
		contexts.push(context);
		context.on_cleanup(() => {
			cleanupOnly();
			cleaned.push(1);
		});
		context.on_cleanup(() => {
			throw error;
		});
		context.on_cleanup(() => cleaned.push(2));
	});
	const report = vi.spyOn(runtime, "reportError").mockImplementation(() => {});
	value.set(1);
	flushEffects();
	cleanupOnly.set(1);
	flushEffects();
	expect(contexts).toHaveLength(2);
	expect(cleaned).toEqual([2, 1]);
	expect(report).toHaveBeenCalledWith(error);
	runtime.page.dispose();
	contexts[1]?.on_cleanup(() => cleaned.push(3));
	expect(cleaned).toEqual([2, 1, 2, 1, 3]);
});

it.each([
	"page",
	"shard",
	"live",
	"navigation",
	"dev",
])("releases and recreates effects exactly once on reused DOM during %s replacement", (kind) => {
	const log: string[] = [];
	const html = `<!--::topcoat::shard::start("/shard", "0", [])-->
			<!--::topcoat::region::start(ab)-->${element}<!--::topcoat::region::end(ab)-->
			<!--::topcoat::shard::end("0")-->`;
	const runtime = mount(html, (context) => {
		log.push("run");
		context.on_cleanup(() => {
			expect(context.element.isConnected).toBe(true);
			log.push("clean");
		});
	});
	const original = document.querySelector("button");
	const shard = runtime.page.contentScope.findRegion("ab")?.scope.unit;
	if (!shard) throw new Error("Missing shard");
	if (kind === "page")
		runtime.page.replaceContent(`<body>${html}</body>`, newRender());
	else if (kind === "shard") shard.replaceContent(element, newRender());
	else if (kind === "live") runtime.page.applySwap("ab", element, null);
	else if (kind === "dev") {
		runtime.page.listenForDevRefresh();
		const detail: DevRuntimeDetail = {};
		window.dispatchEvent(new CustomEvent(DEV_RUNTIME_EVENT, { detail }));
		detail.runtime?.replace(() => {
			const next = new DOMParser().parseFromString(html, "text/html");
			morph(document.body, null, null, next.body.childNodes);
		});
	} else
		runtime.page.replaceDocument(
			new DOMParser().parseFromString(
				`<html><head></head><body>${element}</body></html>`,
				"text/html",
			),
			newRender(),
		);
	flushEffects();
	expect(document.querySelector("button")).toBe(original);
	expect(log).toEqual(["run", "clean", "run"]);
	runtime.page.dispose();
	expect(log).toEqual(["run", "clean", "run", "clean"]);
});

it("keeps nested render ownership when a stale live update arrives", () => {
	const hook = vi.fn();
	const html = `<!--::topcoat::shard::start("/shard", "0", [])-->
		<!--::topcoat::region::start(ab)-->${element}<!--::topcoat::region::end(ab)-->
		<!--::topcoat::shard::end("0")-->`;
	const runtime = mount(html, hook);
	const shard = runtime.page.contentScope.findRegion("ab")?.scope.unit;
	if (!shard) throw new Error("Missing shard");
	shard.replaceContent(
		`<!--::topcoat::region::start(ab)-->${element}<!--::topcoat::region::end(ab)-->`,
		newRender(),
	);
	runtime.page.applySwap("ab", "stale", null);
	flushEffects();
	expect(document.querySelector("button")).not.toBeNull();
	expect(hook).toHaveBeenCalledTimes(2);
});

it("leaves enclosing effects active when only nested content is replaced", () => {
	const log: string[] = [];
	const runtime = mount(
		`<section id="outer" data-topcoat-effect="hook">
		<!--::topcoat::region::start(ab)-->${element}<!--::topcoat::region::end(ab)-->
		</section>`,
		(context) => {
			const id = context.element.id;
			log.push(`run ${id}`);
			context.on_cleanup(() => log.push(`clean ${id}`));
		},
	);
	runtime.page.applySwap("ab", element, null);
	flushEffects();
	expect(log).toEqual(["run outer", "run owned", "clean owned", "run owned"]);
});

it("releases a failed setup and continues initializing other effects", () => {
	const cleanup = vi.fn();
	const report = vi
		.spyOn(Runtime.prototype, "reportError")
		.mockImplementation(() => {});
	let runs = 0;
	mount(`${element}${element}`, (context) => {
		context.on_cleanup(cleanup);
		if (++runs === 1) throw new Error("setup failed");
	});
	expect(runs).toBe(2);
	expect(cleanup).toHaveBeenCalledTimes(1);
	expect(report).toHaveBeenCalledTimes(1);
	report.mockRestore();
});
