// @vitest-environment happy-dom

import { afterEach, expect, it, vi } from "vitest";
import { flushEffects } from "../reactivity";
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

it("parses a connection requirement", () => {
	expect(parseComment(document.createComment("::topcoat::connect"))).toEqual({
		kind: "connect",
	});
	expect(parseComment(document.createComment("::topcoat::connected"))).toBe(
		null,
	);
});

it("parses live region markers", () => {
	expect(
		parseComment(document.createComment("::topcoat::region::start(0a1b)")),
	).toEqual({ kind: "region-start", id: "0a1b" });
	expect(
		parseComment(document.createComment("::topcoat::region::end(0a1b)")),
	).toEqual({ kind: "region-end", id: "0a1b" });
	expect(
		parseComment(document.createComment("::topcoat::region::start(x)")),
	).toBe(null);
});

it("a live region owns its content's signals and reports its dependencies and connection requirement to the unit", () => {
	document.body.innerHTML = `
		<!--::topcoat::signal({"t":"signal","id":"outside","v":1})-->
		<!--::topcoat::region::start(ab)-->
			<!--::topcoat::signal({"t":"signal","id":"1a","v":2})-->
			<!--::topcoat::dep("1a")-->
			<!--::topcoat::connect-->
			<!--::topcoat::region::start(cd)-->
				<!--::topcoat::signal({"t":"signal","id":"nested","v":3})-->
			<!--::topcoat::region::end(cd)-->
		<!--::topcoat::region::end(ab)-->
	`;
	const runtime = new Runtime();
	try {
		runtime.start(document);
		const page = runtime.page.contentScope;
		const outer = page.regions.get("ab");
		if (!outer) throw new Error("Missing outer region");
		const inner = outer.scope.regions.get("cd");
		if (!inner) throw new Error("Missing inner region");

		expect(page.signalIds).toEqual(new Set(["outside"]));
		expect(outer.scope.signalIds).toEqual(new Set(["1a"]));
		expect(inner.scope.signalIds).toEqual(new Set(["nested"]));
		// The page collects dependencies and connection requests from its regions.
		expect(page.dependencies).toEqual(new Set());
		expect(outer.scope.dependencies).toEqual(new Set(["1a"]));
		expect(page.collectDependencies()).toEqual(new Set(["1a"]));
		expect(page.requiresConnection).toBe(false);
		expect(outer.scope.requiresConnection).toBe(true);
		expect(runtime.page.requiresConnection).toBe(true);

		expect(outer.start.data).toBe("::topcoat::region::start(ab)");
		expect(outer.end?.data).toBe("::topcoat::region::end(ab)");
		expect(outer.scope.unit).toBe(runtime.page);
		expect(inner.scope.unit).toBe(runtime.page);
		expect(page.findRegion("cd")).toBe(inner);
		expect(page.regions.has("cd")).toBe(false);
	} finally {
		runtime.page.dispose();
	}
});

it("a region inside a shard belongs to the shard", () => {
	document.body.innerHTML = `
		<!--::topcoat::shard::start("/shards/1", "0", [])-->
			<!--::topcoat::region::start(ab)-->
				<!--::topcoat::signal({"t":"signal","id":"e","v":0})-->
				<!--::topcoat::dep("e")-->
			<!--::topcoat::region::end(ab)-->
		<!--::topcoat::shard::end("0")-->
	`;
	const runtime = new Runtime();
	try {
		runtime.start(document);
		const page = runtime.page.contentScope;
		const region = page.findRegion("ab");
		if (!region) throw new Error("Missing region");

		expect(page.regions.size).toBe(0);
		expect(page.collectDependencies()).toEqual(new Set());
		const shard = region.scope.unit;
		expect(shard).not.toBe(runtime.page);
		expect(shard?.contentScope.collectDependencies()).toEqual(new Set(["e"]));
	} finally {
		runtime.page.dispose();
	}
});

it("unbalanced or mismatched region markers are errors", () => {
	const runtime = new Runtime();
	try {
		document.body.innerHTML = `<!--::topcoat::region::end(ab)-->`;
		expect(() => runtime.start(document)).toThrow("Unbalanced region");

		document.body.innerHTML = `<!--::topcoat::region::start(ab)--><!--::topcoat::region::end(cd)-->`;
		expect(() => runtime.start(document)).toThrow("Mismatched region");
	} finally {
		runtime.page.dispose();
	}
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

it("updates bindings and text after events, then stops both on disposal", async () => {
	document.body.innerHTML = `
		<!--::topcoat::signal({"t":"signal","id":"a","v":1})-->
		<button data-topcoat-on:click="() => cx.signal('a').increment()">add</button>
		<input data-topcoat-bind:value="cx.signal('a').get()">
		<p><!--::topcoat::expr::start("cx.signal('a').get()")-->1<!--::topcoat::expr::end--></p>
	`;
	const runtime = new Runtime();
	try {
		runtime.start(document);
		const button = document.querySelector("button");
		const input = document.querySelector("input");
		const text = document.querySelector("p");
		if (!button || !input || !text) throw new Error("Missing fixture elements");
		expect(input.value).toBe("1");
		expect(text.textContent).toBe("1");
		const count = runtime.context.signal("a");
		button.click();
		button.click();
		expect(input.value).toBe("1");
		expect(text.textContent).toBe("1");
		await Promise.resolve();
		expect(input.value).toBe("3");
		expect(text.textContent).toBe("3");

		button.click();
		runtime.page.dispose();
		await Promise.resolve();
		button.click();
		expect((count.get() as F64).dehydrate()).toBe(4);
		count.set(new F64(5));
		await Promise.resolve();
		expect(input.value).toBe("3");
		expect(text.textContent).toBe("3");
	} finally {
		runtime.page.dispose();
	}
});

it("a page replacement releases nested shards and adopts surviving signals", async () => {
	document.body.innerHTML = `
		<!--::topcoat::signal({"t":"signal","id":"a","v":1})-->
		<!--::topcoat::shard::start("/shards/1", "0", [])-->
			<!--::topcoat::shard::start("/shards/2", "1", [])-->
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
	const snapshot = (html: string) =>
		`${JSON.stringify({ t: "snapshot", html })}\n`;
	const fetch = vi.fn((url: string, _init: RequestInit) =>
		url.includes("/shards/")
			? pendingShard
			: Promise.resolve(
					new Response(
						snapshot(`
						<body>
							<!--::topcoat::signal({"t":"signal","id":"a","v":99})-->
							<p>replacement</p>
						</body>
					`),
					),
				),
	);
	vi.stubGlobal("fetch", fetch);
	const runtime = new Runtime();
	try {
		runtime.start(document);
		runtime.context.signal("a").set(new F64(7));
		runtime.context.signal("b").set(new F64(1));
		flushEffects();
		await Promise.resolve();

		expect(fetch).toHaveBeenCalledTimes(1);
		expect(fetch.mock.calls[0]?.[0]).toContain("/shards/2");
		const childRequest = fetch.mock.calls[0]?.[1].signal;
		await runtime.page.refresh();

		expect(childRequest?.aborted).toBe(true);
		expect(runtime.registry.has("b")).toBe(false);
		expect((runtime.registry.read("a") as F64).dehydrate()).toBe(7);
		expect(runtime.page.contentScope.children.size).toBe(0);

		finishShard(new Response(snapshot("<p>stale shard</p>")));
		await new Promise((resolve) => setTimeout(resolve, 0));
		expect(document.querySelector("p")?.textContent).toBe("replacement");
	} finally {
		runtime.page.dispose();
	}
});

it("keeps checkbox signals in sync through repeated clicks", () => {
	document.body.innerHTML = `
		<!--::topcoat::signal({"t":"signal","id":"checked","v":false})-->
		<input type="checkbox"
			data-topcoat-bind:checked="cx.signal('checked').get()"
			data-topcoat-on:change="e => cx.signal('checked').set(e.target.checked)">
	`;
	const runtime = new Runtime();
	try {
		runtime.start(document);
		const input = document.querySelector("input");
		if (!input) throw new Error("Missing checkbox");
		expect(input.checked).toBe(false);

		for (const checked of [true, false, true, false]) {
			input.click();
			flushEffects();
			expect(input.checked).toBe(checked);
			expect(input.hasAttribute("checked")).toBe(checked);
			expect(runtime.context.signal("checked").get()).toEqual(
				runtime.context.hydrate(checked),
			);
		}
	} finally {
		runtime.page.dispose();
	}
});

it("selects only the radio matching the signal after each click", () => {
	document.body.innerHTML = `
		<!--::topcoat::signal({"t":"signal","id":"range","v":"week"})-->
		${["day", "week", "month"]
			.map(
				(value) => `
			<input type="radio" name="range" value="${value}"
				data-topcoat-bind:checked="cx.signal('range').get().eq(cx.hydrate('${value}'))"
				data-topcoat-on:change="e => cx.signal('range').set(e.target.value)">
		`,
			)
			.join("")}
	`;
	const runtime = new Runtime();
	try {
		runtime.start(document);
		const inputs = Array.from(document.querySelectorAll("input"));
		expect(inputs.map((input) => input.checked)).toEqual([false, true, false]);

		for (const index of [0, 2, 1]) {
			inputs[index]?.click();
			flushEffects();
			expect(inputs.map((input) => input.checked)).toEqual(
				inputs.map((_, candidate) => candidate === index),
			);
		}
	} finally {
		runtime.page.dispose();
	}
});

it("clears the live input value and removes its attribute for an absent value", () => {
	document.body.innerHTML = `
		<!--::topcoat::signal({"t":"signal","id":"value","v":{"t":"Option","v":"initial"}})-->
		<input value="initial" data-topcoat-bind:value="cx.signal('value').get()">
	`;
	const runtime = new Runtime();
	try {
		runtime.start(document);
		const input = document.querySelector("input");
		if (!input) throw new Error("Missing input");
		expect(input.value).toBe("initial");
		input.value = "edited";

		const value = runtime.context.signal("value");
		value.set(runtime.context.none());
		flushEffects();
		expect(input.value).toBe("");
		// happy-dom's named lookup becomes stale alongside data-topcoat-bind:value.
		expect(input.getAttributeNames()).not.toContain("value");

		value.set(runtime.context.some(runtime.context.hydrate("restored")));
		flushEffects();
		expect(input.value).toBe("restored");
		expect(
			Array.from(input.attributes).find((attr) => attr.name === "value")?.value,
		).toBe("restored");
	} finally {
		runtime.page.dispose();
	}
});
