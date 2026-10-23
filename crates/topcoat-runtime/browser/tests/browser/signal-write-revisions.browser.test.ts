import { afterEach, beforeEach, expect, inject, it, vi } from "vitest";
import { parseComment } from "../../src/dom/markers";
import { compile } from "../../src/expression/compile";
import { untrack } from "../../src/reactivity";
import { applyFrame, newRender } from "../../src/render/frames";
import { Runtime } from "../../src/runtime";
import { Scope } from "../../src/scope";
import type { Bool, Integer, WriteSignal } from "../../src/surrogate";

let runtime: Runtime;
let html: string;

beforeEach(() => {
	html = inject("revisionFixture");
	document.body.innerHTML = html;
	runtime = new Runtime();
	runtime.start(document.body);
});

afterEach(() => {
	runtime.navigation.dispose();
	runtime.page.dispose();
	document.body.replaceChildren();
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
});

function element<T extends HTMLElement>(selector: string): T {
	const found = document.querySelector<T>(selector);
	if (!found) throw new Error(`Missing fixture element: ${selector}`);
	return found;
}

function expression<T>(name: string): T {
	const source = element("#checkpoint").getAttribute(`data-${name}`);
	if (source === null) throw new Error(`Missing compiled expression: ${name}`);
	return compile<T>(source, name)(runtime.context);
}

// Full page responses keep declarations and region markers inside the body.
function page(content: string): string {
	return `<!doctype html><html><head></head><body>${content}</body></html>`;
}

function handle() {
	const [id] = Object.keys(JSON.parse(runtime.page.signalBody()).signals);
	if (id === undefined) throw new Error("Missing signal declaration");
	return runtime.registry.handle(id);
}

it("runs compiled u64 reads and comparisons through aliases without changing transport", () => {
	const read = expression<() => Integer>("read");
	const alias = expression<() => Integer>("alias-read");
	const changed = expression<(checkpoint: Integer) => Bool>("changed");
	const checkpoint = read();
	const originalBody = runtime.page.signalBody();
	const originalDeclaration = document.body.firstChild?.textContent;
	expect(checkpoint.dehydrate()).toEqual({ t: "u64", bits: 64, v: "0" });
	element<HTMLButtonElement>("#same").click();
	expect(read().dehydrate()).toEqual({ t: "u64", bits: 64, v: "1" });
	expect(alias().eq(read()).dehydrate()).toBe(true);
	expect(changed(checkpoint).dehydrate()).toBe(true);
	expect(changed(read()).dehydrate()).toBe(false);
	expect(runtime.page.signalBody()).toBe(originalBody);
	expect(document.body.firstChild?.textContent).toBe(originalDeclaration);
	const [id] = Object.keys(JSON.parse(originalBody).signals);
	if (id === undefined) throw new Error("Missing declaration");
	const declaration = document.body.firstChild;
	if (!(declaration instanceof Comment))
		throw new Error("Missing declaration comment");
	expect(parseComment(declaration)).toEqual({
		kind: "signal",
		id,
		value: false,
	});
	expect(runtime.page.contentScope.signalIds).toEqual(new Set([id]));
	expect(runtime.context.signal(id).dehydrate()).toEqual({
		t: "Signal",
		id,
		v: false,
	});
	expect(handle().write_revision()).toBe(1n);
});

it.each([
	["same", false],
	["same", true],
	["round-trip", false],
	["round-trip", true],
] as const)("distinguishes %s writes before and after a native checkpoint (morph=%s)", async (write, morph) => {
	const read = expression<() => Integer>("read");
	const changed = expression<(checkpoint: Integer) => Bool>("changed");
	const checkpoints: Integer[] = [];
	const form = element<HTMLFormElement>("form");
	form.addEventListener("reset", () => untrack(() => checkpoints.push(read())));
	const button = element<HTMLButtonElement>(`#${write}`);
	button.click();
	form.reset();
	if (morph) runtime.page.replaceContent(page(html), newRender());
	const earlier = checkpoints[0];
	if (!earlier) throw new Error("Native reset did not dispatch");
	expect(changed(earlier).dehydrate()).toBe(false);
	form.reset();
	const later = checkpoints[1];
	if (!later) throw new Error("Native reset did not dispatch");
	button.click();
	if (morph) runtime.page.replaceContent(page(html), newRender());
	expect(changed(later).dehydrate()).toBe(true);
	await Promise.resolve();
	expect(element("#value").textContent).toBe("false");
	expect(read().toString()).toBe(write === "same" ? "2" : "4");
});

it("keeps a synchronous reset listener's checkpoint untracked inside an effect", async () => {
	const read = expression<() => Integer>("read");
	const form = element<HTMLFormElement>("form");
	form.addEventListener("reset", () => untrack(read));
	const callback = vi.fn(() => form.reset());
	runtime.page.contentScope.effect(callback);
	element<HTMLButtonElement>("#same").click();
	await Promise.resolve();
	expect(callback).toHaveBeenCalledTimes(1);
});

it.each([
	"region",
	"page",
	"document",
])("preserves revision checkpoints and disposes old subscriptions during %s adoption", async (kind) => {
	if (kind === "region") {
		runtime.page.replaceContent(
			page(
				`<!--::topcoat::region::start(ab)-->${html}<!--::topcoat::region::end(ab)-->`,
			),
			newRender(),
		);
	}
	const oldScope =
		kind === "region"
			? runtime.page.contentScope.findRegion("ab")?.scope
			: runtime.page.contentScope;
	if (!oldScope) throw new Error("Missing owner");
	const previous = handle();
	const read = expression<() => Integer>("read");
	const before = read();
	const oldRuns = vi.fn(() => read());
	oldScope.effect(oldRuns);
	const node = element("#checkpoint");
	element<HTMLButtonElement>("#round-trip").click();
	if (kind === "region") {
		runtime.page.applySwap("ab", html, oldScope.render);
	} else if (kind === "page") {
		runtime.page.replaceContent(page(html), newRender());
	} else {
		const next = new DOMParser().parseFromString(
			document.documentElement.outerHTML,
			"text/html",
		);
		next.body.innerHTML = html;
		runtime.page.replaceDocument(next, newRender());
	}
	const adopted = handle();
	expect(adopted).toBe(previous);
	const [id] = Object.keys(JSON.parse(runtime.page.signalBody()).signals);
	expect(id).toBeDefined();
	const owner =
		kind === "region"
			? runtime.page.contentScope.findRegion("ab")?.scope
			: runtime.page.contentScope;
	expect(owner?.signalIds).toEqual(new Set([id]));
	expect(adopted.write_revision()).toBe(2n);
	expect(element("#checkpoint")).toBe(node);
	expect(
		expression<(checkpoint: Integer) => Bool>("changed")(before).dehydrate(),
	).toBe(true);
	await Promise.resolve();
	expect(oldRuns).toHaveBeenCalledTimes(1);
	expect(oldScope.isDisposed).toBe(true);
	element<HTMLButtonElement>("#same").click();
	await Promise.resolve();
	expect(adopted.write_revision()).toBe(3n);
	expect(oldRuns).toHaveBeenCalledTimes(1);
});

it("ignores stale and failed frames without resetting the live handle", () => {
	const render = newRender();
	runtime.page.replaceContent(
		page(
			`<!--::topcoat::region::start(ab)-->${html}<!--::topcoat::region::end(ab)-->`,
		),
		render,
	);
	const previous = handle();
	element<HTMLButtonElement>("#same").click();
	runtime.page.applySwap("ab", "removed", newRender());
	runtime.page.applySwap("missing", "removed", render);
	expect(() =>
		applyFrame(runtime.page, { t: "error", status: 500 }, "Page", render),
	).toThrow();
	expect(handle()).toBe(previous);
	expect(previous.write_revision()).toBe(1n);
});

it("failed requests leave the declared handle and its revision intact", async () => {
	vi.stubGlobal(
		"fetch",
		vi.fn(async () => new Response("", { status: 500 })),
	);
	const previous = handle();
	element<HTMLButtonElement>("#same").click();
	await expect(runtime.page.refresh()).rejects.toThrow();
	expect(handle()).toBe(previous);
	expect(previous.write_revision()).toBe(1n);
});

it("removal and recreation create a fresh lifetime isolated from old aliases", async () => {
	const previous = handle();
	const alias = expression<() => WriteSignal<Bool>>("capture")();
	element<HTMLButtonElement>("#same").click();
	runtime.page.replaceContent("<p>removed</p>", newRender());
	expect(runtime.page.signalBody()).toBe('{"signals":{}}');
	runtime.page.replaceContent(page(html), newRender());
	const fresh = handle();
	expect(fresh).not.toBe(previous);
	expect(fresh.write_revision()).toBe(0n);
	previous.set(previous());
	await Promise.resolve();
	expect(alias.write_revision().toString()).toBe("2");
	expect(fresh.write_revision()).toBe(0n);
	element<HTMLButtonElement>("#same").click();
	expect(fresh.write_revision()).toBe(1n);
});

it("retains counters during reentrant declaration transfer through real scopes", () => {
	const previous = handle();
	element<HTMLButtonElement>("#same").click();
	const orphans = runtime.page.contentScope.release();
	const intermediate = new Scope(null, runtime);
	const finalOwner = new Scope(null, runtime);
	try {
		runtime.hydrate(document.body, null, null, intermediate, orphans);
		intermediate.effect(() => {
			previous.write_revision();
			const transfer = intermediate.release();
			runtime.hydrate(document.body, null, null, finalOwner, transfer);
			expect(transfer.size).toBe(0);
		});
		expect(orphans.size).toBe(0);
		intermediate.dispose();
		expect(previous.write_revision()).toBe(1n);
		expect(Object.values(finalOwner.collectSignalValues())).toEqual([false]);
		const [id] = finalOwner.signalIds;
		if (!id) throw new Error("Missing adopted signal");
		expect(runtime.registry.handle(id)).toBe(previous);
	} finally {
		intermediate.dispose();
		finalOwner.dispose();
	}
});

it("ignored declarations and hydration reads do not count as commits", () => {
	const previous = handle();
	const [id] = runtime.page.contentScope.signalIds;
	if (!id) throw new Error("Missing signal");
	element<HTMLButtonElement>("#same").click();
	expect(runtime.registry.insert(id, true)).toBe(false);
	const duplicate = new Scope(null, runtime);
	const fragment = document.createElement("div");
	fragment.innerHTML = html;
	try {
		runtime.hydrate(fragment, null, null, duplicate);
		expect(duplicate.signalIds.size).toBe(0);
		expect(previous.write_revision()).toBe(1n);
		duplicate.dispose();
		expect(handle()).toBe(previous);
	} finally {
		duplicate.dispose();
	}
});

it("revision-only invalidation does not turn a value watcher into a server request", async () => {
	const fetch = vi.fn();
	vi.stubGlobal("fetch", fetch);
	const [id] = runtime.page.contentScope.signalIds;
	if (!id) throw new Error("Missing signal");
	runtime.page.contentScope.dependencies.add(id);
	runtime.page.resubscribe();
	const previous = handle();
	const seen: bigint[] = [];
	runtime.page.contentScope.effect(() => seen.push(previous.write_revision()));
	previous.set(previous());
	await new Promise((resolve) => setTimeout(resolve, 0));
	expect(seen).toEqual([0n, 1n]);
	expect(fetch).not.toHaveBeenCalled();
});
