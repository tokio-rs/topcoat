// @vitest-environment happy-dom

import { afterEach, expect, it, vi } from "vitest";
import { flushEffects } from "../reactivity";
import { RUNTIME_HEADER } from "../render/request";
import { Runtime } from "../runtime";
import type { F64 } from "../surrogate";

type Log = { log: string[] };

const globals = globalThis as typeof globalThis & Log;

let runtime: Runtime | null = null;

afterEach(() => {
	runtime?.navigation.dispose();
	runtime?.page.dispose();
	runtime = null;
	vi.unstubAllGlobals();
	vi.restoreAllMocks();
	sessionStorage.clear();
	document.body.innerHTML = "";
});

const declaration = (id: string, value: number) =>
	`<!--::topcoat::signal({"t":"signal","id":"${id}","v":${value}})-->`;

/** A mount handler that records `name` in `globalThis.log`. */
const logs = (name: string) =>
	`data-topcoat-on:mount="(e) => globalThis.log.push('${name}:' + e.event_type)"`;

function start(body: string): Runtime {
	globals.log = [];
	document.body.innerHTML = body;
	runtime = new Runtime();
	runtime.start(document);
	return runtime;
}

/** Lets queued mount events, requests, and effects run. */
async function settle(): Promise<void> {
	for (let i = 0; i < 5; i += 1) {
		flushEffects();
		await new Promise((resolve) => setTimeout(resolve, 0));
	}
	flushEffects();
}

it("runs after hydration, once every signal in the document is declared", async () => {
	const rt = start(`
		<main data-topcoat-on:mount="() => cx.signal('late').increment()"></main>
		${declaration("late", 1)}
		<p><!--::topcoat::expr::start("cx.signal('late').get()")-->1<!--::topcoat::expr::end--></p>
	`);
	const count = () => (rt.context.signal("late").get() as F64).dehydrate();

	// Hydration itself does not run the handler.
	expect(count()).toBe(1);
	await settle();
	expect(count()).toBe(2);
	expect(document.querySelector("p")?.textContent).toBe("2");
	await settle();
	expect(count()).toBe(2);
});

it("dispatches in document order without reaching ancestors' handlers", async () => {
	start(`<div ${logs("outer")}><span ${logs("inner")}></span></div>`);
	await settle();
	expect(globals.log).toEqual(["outer:mount", "inner:mount"]);
});

it("runs again for content that a server render replaces, but not outside it", async () => {
	const rt = start(`
		<p ${logs("page")}></p>
		<!--::topcoat::region::start(ab)-->
			<p ${logs("region")}></p>
		<!--::topcoat::region::end(ab)-->
	`);
	await settle();
	rt.page.applySwap("ab", `<p ${logs("swapped")}></p>`, null);
	await settle();
	expect(globals.log).toEqual(["page:mount", "region:mount", "swapped:mount"]);
});

it("skips content that is replaced before its handler runs", async () => {
	const rt = start(`
		<!--::topcoat::region::start(ab)-->
			<p ${logs("first")}></p>
		<!--::topcoat::region::end(ab)-->
	`);
	rt.page.applySwap("ab", `<p ${logs("second")}></p>`, null);
	await settle();
	expect(globals.log).toEqual(["second:mount"]);
});

it("reruns the page when a handler changes a signal the server read", async () => {
	const fetch = vi.fn(async () => new Promise<Response>(() => {}));
	vi.stubGlobal("fetch", fetch);
	start(`
		${declaration("a", 1)}
		<!--::topcoat::dep("a")-->
		<main data-topcoat-on:mount="() => cx.signal('a').increment()"></main>
	`);
	await settle();
	expect(fetch).toHaveBeenCalledTimes(1);
	expect(fetch).toHaveBeenCalledWith(
		expect.any(String),
		expect.objectContaining({
			headers: expect.objectContaining({ [RUNTIME_HEADER]: "true" }),
		}),
	);
});

/** Escapes `js` for a double-quoted attribute, as the server renders it. */
const attribute = (js: string) =>
	js.replaceAll("&", "&amp;").replaceAll('"', "&quot;");

const signal = (id: string) => `(cx.hydrate({"t":"Signal","id":"${id}"}))`;

it("formats times, shows a saved message, and resumes an interrupted operation on load", async () => {
	// The handlers are the compiled output of the runtime example's `mount` page.
	const formatTimes = `(__local0) => { let __local1 = document.querySelectorAll('time[data-ms]').forEach((time) => {
		time.textContent = new Date(Number(time.dataset.ms)).toLocaleString();
	});  }`;
	const showFlash = `(__local0) => ${signal("flash")}.set((() => {
		const message = sessionStorage.getItem('flash') ?? '';
		sessionStorage.removeItem('flash');
		return cx.hydrate(message);
	})())`;
	const moveOne = `(cx.hydrate({"t":"Procedure","path":"/procedures/move_one"}))`;
	const resume = `async (__local0) => { let __local1 = cx.hydrate(sessionStorage.getItem('moved') !== null); return (await (async () => { if (__local1.dehydrate()) { ${signal("moved")}.set(cx.hydrate(Number(sessionStorage.getItem('moved')))); while (${signal("moved")}.get().lt(cx.hydrate(5.0)).dehydrate()) { let __local2 = (await ${moveOne}.call(${signal("moved")}.get())); let __local3 = sessionStorage.setItem('moved', String(__local2)); ${signal("moved")}.set(__local2);  }; let __local4 = sessionStorage.removeItem('moved');  } })()); }`;

	const moves: unknown[] = [];
	vi.stubGlobal("fetch", async (_url: string, init: RequestInit) => {
		const [moved] = JSON.parse(String(init.body)) as [number];
		moves.push(moved);
		return new Response(JSON.stringify(moved + 1));
	});
	sessionStorage.setItem("flash", "username is now ada");
	sessionStorage.setItem("moved", "3");
	const ms = Date.UTC(2026, 9, 20, 12, 30);
	const rt = start(`
		<!--::topcoat::signal({"t":"signal","id":"flash","v":""})-->
		${declaration("moved", 0)}
		<section data-topcoat-on:mount="${attribute(formatTimes)}"><time data-ms="${ms}">${ms}</time></section>
		<section data-topcoat-on:mount="${attribute(showFlash)}"></section>
		<section data-topcoat-on:mount="${attribute(resume)}"></section>
	`);
	await settle();

	expect(document.querySelector("time")?.textContent).toBe(
		new Date(ms).toLocaleString(),
	);
	expect(String(rt.context.signal("flash").get())).toBe("username is now ada");
	expect(sessionStorage.getItem("flash")).toBe(null);
	// The operation continues from the saved progress instead of starting over.
	expect(moves).toEqual([3, 4]);
	expect((rt.context.signal("moved").get() as F64).dehydrate()).toBe(5);
	expect(sessionStorage.getItem("moved")).toBe(null);
});
