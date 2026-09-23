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

		finishShard(new Response("<p>stale shard</p>"));
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
