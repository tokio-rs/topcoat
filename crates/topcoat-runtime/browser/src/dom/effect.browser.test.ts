import { afterEach, expect, it, vi } from "vitest";
import { flushEffects, signal } from "../reactivity";
import { newRender } from "../render/frames";
import { Runtime } from "../runtime";
import { type Event, EventTarget, F64 } from "../surrogate";
import { Str } from "../surrogate/string";
import type { EffectContext } from "./effect";

let runtime: Runtime;

function mount(html: string, hook: (context: EffectContext) => void): void {
	document.body.innerHTML = html;
	vi.stubGlobal("hook", hook);
	runtime = new Runtime();
	runtime.start(document);
	flushEffects();
}

const control =
	'<input id="control" form="external" data-topcoat-effect="hook">';
const live = `<!--::topcoat::region::start(ab)-->${control}<!--::topcoat::region::end(ab)-->`;

afterEach(() => {
	runtime?.page.dispose();
	flushEffects();
	vi.unstubAllGlobals();
	document.body.innerHTML = "";
});

it.each([
	["attribute", "focus"],
	["attribute", "click"],
	["owned", "focus"],
	["owned", "click"],
])("keeps %s %s handler reads out of the dispatching effect", (attachment, name) => {
	let runs = 0;
	let cleanups = 0;
	const events: string[] = [];
	const handler = (event: Event) => {
		runtime.context.signal("handler").get();
		events.push(`${event.event_type}:${event.current_target.id}`);
	};
	vi.stubGlobal("onEvent", handler);
	mount(
		`<!--::topcoat::signal({"t":"signal","id":"handler","v":0})-->
		<!--::topcoat::signal({"t":"signal","id":"setup","v":0})-->
		<input id="attached" data-topcoat-effect="hook"
			${attachment === "attribute" ? `data-topcoat-on:${name}="onEvent"` : ""}>
		<input id="other">`,
		(context) => {
			runs++;
			context.on_cleanup(() => cleanups++);
			if (attachment === "owned") {
				context.listen(context.target(), new Str(name), handler);
			}
			const input = context.element as HTMLInputElement;
			input.focus();
			if (name === "click") input.click();
			// Tracking must resume after the synchronous event returns.
			runtime.context.signal("setup").get();
		},
	);
	expect(events).toEqual([`${name}:attached`]);
	document.querySelector<HTMLInputElement>("#other")?.focus();
	expect(document.activeElement?.id).toBe("other");
	runtime.context.signal("handler").set(new F64(1));
	flushEffects();
	expect({
		runs,
		cleanups,
		focused: document.activeElement?.id,
		events,
	}).toEqual({
		runs: 1,
		cleanups: 0,
		focused: "other",
		events: [`${name}:attached`],
	});

	runtime.context.signal("setup").set(new F64(1));
	flushEffects();
	expect({
		runs,
		cleanups,
		focused: document.activeElement?.id,
		events,
	}).toEqual({
		runs: 2,
		cleanups: 1,
		focused: "attached",
		events: [`${name}:attached`, `${name}:attached`],
	});
});

it("owns external form reset listeners across reruns, reused elements and live removal", () => {
	const value = signal(0);
	const resetCount = signal(0);
	const resetTargets: string[] = [];
	let runs = 0;
	const hook = (context: EffectContext) => {
		value();
		runs++;
		const input = context.element as HTMLInputElement;
		expect(input.isConnected).toBe(true);
		context.listen(new EventTarget(input.form), new Str("reset"), (event) => {
			resetTargets.push(event.current_target.id.toString());
			resetCount.set(resetCount() + 1);
			event.prevent_default();
		});
	};
	mount(`<form id="external"></form>${live}`, hook);
	const input = document.querySelector("input");
	const form = document.querySelector("form");
	if (!form) throw new Error("Missing external form");
	form.reset();
	flushEffects();
	expect(runs).toBe(1);
	value.set(1);
	flushEffects();
	form.reset();
	runtime.page.applySwap("ab", control, null);
	flushEffects();
	expect(document.querySelector("input")).toBe(input);
	form.reset();
	expect(runs).toBe(3);
	expect(resetCount()).toBe(3);
	expect(resetTargets).toEqual(["external", "external", "external"]);
	runtime.page.applySwap("ab", "", null);
	form.reset();
	expect(resetCount()).toBe(3);
});

it("runs compiled signal reads after hydration and disconnects native observers before rerun", async () => {
	let observations = 0;
	const runs: number[] = [];
	const cleaned: number[] = [];
	mount(
		`<section id="observed" data-topcoat-effect="scope => { cx.signal('a').get(); hook(scope); }">
		<!--::topcoat::signal({"t":"signal","id":"a","v":1})-->
		<input data-topcoat-bind:value="cx.signal('a').get()">
		</section>`,
		(context) => {
			const value = (runtime.context.signal("a").get() as F64).dehydrate();
			runs.push(value);
			expect(context.element.querySelector("input")?.value).toBe(String(value));
			const observer = new MutationObserver(() => observations++);
			observer.observe(context.element, { attributes: true });
			context.on_cleanup(() => {
				observer.disconnect();
				cleaned.push(value);
			});
		},
	);
	const observed = document.querySelector("section");
	if (!observed) throw new Error("Missing observed element");
	observed.setAttribute("data-size", "one");
	await Promise.resolve();
	expect(observations).toBe(1);
	observed.setAttribute("data-size", "queued");
	runtime.context.signal("a").set(new F64(2));
	flushEffects();
	await Promise.resolve();
	expect(observations).toBe(1);
	expect(runs).toEqual([1, 2]);
	expect(cleaned).toEqual([1]);
	observed.setAttribute("data-size", "queued again");
	runtime.page.replaceDocument(
		new DOMParser().parseFromString(
			"<html><head></head><body>Next page</body></html>",
			"text/html",
		),
		newRender(),
	);
	await Promise.resolve();
	expect(cleaned).toEqual([1, 2]);
	expect(observations).toBe(1);
});

it("invalidates deferred work and releases other resources when cleanup throws", async () => {
	const value = signal(0);
	const completed: number[] = [];
	let clicks = 0;
	let released = 0;
	let saved: EffectContext | undefined;
	mount(control, (context) => {
		saved = context;
		const version = value();
		context.listen(context.target(), new Str("click"), () => clicks++);
		context.on_cleanup(() => released++);
		context.on_cleanup(() => {
			throw new Error("cleanup failure");
		});
		queueMicrotask(() => {
			if (context.is_active().dehydrate()) completed.push(version);
		});
	});
	const report = vi.spyOn(runtime, "reportError").mockImplementation(() => {});
	const target = document.querySelector("input");
	value.set(1);
	flushEffects();
	runtime.page.dispose();
	saved?.listen(new EventTarget(target), new Str("click"), () => clicks++);
	target?.click();
	await Promise.resolve();
	expect(completed).toEqual([]);
	expect(clicks).toBe(0);
	expect(released).toBe(2);
	expect(report).toHaveBeenCalledTimes(2);
});
