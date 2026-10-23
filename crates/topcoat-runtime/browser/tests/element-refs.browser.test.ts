import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { compile } from "../src/expression/compile";
import { dehydrate } from "../src/expression/dehydrate";
import { newRender } from "../src/render/frames";
import { ShardUnit } from "../src/render/shard";
import { Runtime } from "../src/runtime";
import type { Bool } from "../src/surrogate/bool";
import {
	type ElementHandle,
	ElementRef,
	type ElementRefError,
} from "../src/surrogate/element-ref";
import type { Result } from "../src/surrogate/result";
import fixture from "./element-refs.json";

let runtime: Runtime;
let originalHead: string;
let originalUrl: string;

beforeEach(() => {
	originalHead = document.head.innerHTML;
	originalUrl = location.href;
	document.body.innerHTML = "";
	runtime = new Runtime();
});

afterEach(() => {
	runtime.navigation.dispose();
	runtime.page.dispose();
	vi.restoreAllMocks();
	vi.unstubAllGlobals();
	document.head.innerHTML = originalHead;
	document.body.innerHTML = "";
	history.replaceState(null, "", originalUrl);
});

function resolver(name: "input" | "dialog" | "html") {
	return compile<() => Result<ElementHandle, ElementRefError>>(
		fixture[name],
		name,
	)(runtime.context);
}

it("executes Rust-generated captures with explicit missing and wrong-type outcomes", () => {
	const resolve = resolver("input");
	const missing = compile<() => Bool>(
		fixture.missing,
		"missing",
	)(runtime.context);
	const wrongType = compile<() => Bool>(
		fixture.wrong_type,
		"wrong type",
	)(runtime.context);
	expect(missing().dehydrate()).toBe(true);
	expect(wrongType().dehydrate()).toBe(false);
	document.body.innerHTML = '<div id="tc-field"></div>';
	expect(missing().dehydrate()).toBe(false);
	expect(wrongType().dehydrate()).toBe(true);
	document.body.innerHTML = '<input id="tc-field">';
	expect(resolve().unwrap().node).toBe(document.getElementById("tc-field"));
	expect(String(compile(fixture.id, "id")(runtime.context))).toBe("tc-field");
});

it("checks native HTML types, including dialog and the general HTML handle", () => {
	document.body.innerHTML =
		'<dialog id="tc-dialog"></dialog><section id="tc-container"></section>';
	expect(resolver("dialog")().unwrap().node).toBeInstanceOf(HTMLDialogElement);
	expect(resolver("html")().unwrap().node).toBeInstanceOf(HTMLElement);
	document.body.innerHTML =
		'<input id="tc-dialog"><svg id="tc-container"></svg>';
	expect(resolver("dialog")().unwrap_err().is_wrong_type().dehydrate()).toBe(
		true,
	);
	expect(resolver("html")().unwrap_err().is_wrong_type().dehydrate()).toBe(
		true,
	);
});

it("finds a matching replacement and reports removal without retaining the old node", () => {
	document.body.innerHTML = '<input id="tc-field">';
	const resolve = resolver("input");
	const old = resolve().unwrap().node;
	const replacement = document.createElement("input");
	replacement.id = old.id;
	old.replaceWith(replacement);
	expect(resolve().unwrap().node).toBe(replacement);
	expect(old.isConnected).toBe(false);
	replacement.remove();
	expect(resolve().unwrap_err().is_missing().dehydrate()).toBe(true);
	document.body.append(old);
	expect(resolve().unwrap().node).toBe(old);
});

it("serializes reference metadata but refuses resolved nodes in requests and signals", () => {
	document.body.innerHTML = '<input id="tc-field">';
	const reference = new ElementRef("tc-field", "input");
	expect(dehydrate(reference.clone())).toEqual({
		t: "ElementRef",
		id: "tc-field",
		kind: "input",
	});
	const resolved = reference.resolve().unwrap();
	expect(() => dehydrate(resolved)).toThrow();
	expect(() => dehydrate(reference.resolve())).toThrow();
	runtime.registry.insert("element", resolved);
	expect(() => dehydrate(runtime.context.signal("element"))).toThrow();
});

it("keeps keyed component IDs and captures paired across reorder and removal", () => {
	document.body.innerHTML = fixture.keyed;
	runtime.start(document);
	const inputs = Array.from(document.querySelectorAll("input"));
	expect(new Set(inputs.map((input) => input.id)).size).toBe(2);
	const references = inputs.map((input) => new ElementRef(input.id, "input"));
	const lookup = vi.spyOn(document, "getElementById");
	const clickFields = () => {
		for (const button of document.querySelectorAll("button")) {
			button.click();
			const key = button.dataset.key;
			expect(lookup).toHaveBeenLastCalledWith(
				document.querySelector<HTMLInputElement>(`input[data-key="${key}"]`)
					?.id,
			);
		}
	};
	clickFields();
	runtime.page.replaceContent(
		`<html><body>${fixture.reordered}</body></html>`,
		newRender(),
	);
	clickFields();
	expect(Array.from(document.querySelectorAll("input"))).toEqual(
		[...inputs].reverse(),
	);
	for (const [index, reference] of references.entries()) {
		expect(reference.resolve().unwrap().node).toBe(inputs[index]);
	}
	inputs[0]?.remove();
	expect(references[0]?.resolve().unwrap_err().is_missing().dehydrate()).toBe(
		true,
	);
	expect(references[1]?.resolve().is_ok().dehydrate()).toBe(true);
});

it("resolves current nodes after shard responses and live-region swaps", async () => {
	document.body.innerHTML =
		'<!--::topcoat::shard::start("/shards/field", "0", [])--><input id="tc-field"><!--::topcoat::shard::end("0")-->';
	runtime.start(document);
	const shard = Array.from(runtime.page.contentScope.children)
		.map((scope) => scope.unit)
		.find((unit) => unit instanceof ShardUnit);
	if (!shard) throw new Error("Missing shard");
	const resolve = resolver("input");
	const original = resolve().unwrap().node;
	const response = (html: string) =>
		new Response(`${JSON.stringify({ t: "snapshot", html })}\n`, {
			headers: { "Content-Type": "application/x-ndjson" },
		});
	vi.stubGlobal("fetch", async () => response('<div id="tc-field"></div>'));
	await shard.refresh();
	expect(original.isConnected).toBe(false);
	expect(resolve().unwrap_err().is_wrong_type().dehydrate()).toBe(true);
	vi.stubGlobal("fetch", async () =>
		response(
			'<!--::topcoat::region::start(ab)--><input id="tc-field"><!--::topcoat::region::end(ab)-->',
		),
	);
	await shard.refresh();
	const replacement = resolve().unwrap().node;
	expect(replacement).not.toBe(original);
	shard.applySwap("ab", "", shard.contentScope.render);
	expect(resolve().unwrap_err().is_missing().dehydrate()).toBe(true);
	shard.applySwap("ab", '<input id="tc-field">', shard.contentScope.render);
	expect(resolve().unwrap().node).not.toBe(replacement);
});

it("uses current document lookup through runtime navigation away and back", async () => {
	document.body.innerHTML =
		'<input id="tc-field"><a data-topcoat-link="never" href="/element-ref-next">Next</a>';
	runtime.start(document);
	const resolve = resolver("input");
	const original = resolve().unwrap().node;
	const next = (body: string) =>
		new Response(
			`${JSON.stringify({ t: "snapshot", html: `<!DOCTYPE html><html><head>${originalHead}</head><body>${body}</body></html>` })}\n`,
			{ headers: { "Content-Type": "application/x-ndjson" } },
		);
	vi.stubGlobal("fetch", async () =>
		next('<a data-topcoat-link="never" href="/element-ref-back">Back</a>'),
	);
	document.querySelector<HTMLAnchorElement>("a")?.click();
	await runtime.navigation.pending;
	expect(location.pathname).toBe("/element-ref-next");
	expect(resolve().unwrap_err().is_missing().dehydrate()).toBe(true);
	vi.stubGlobal("fetch", async () => next('<input id="tc-field">'));
	document.querySelector<HTMLAnchorElement>("a")?.click();
	await runtime.navigation.pending;
	expect(location.pathname).toBe("/element-ref-back");
	expect(resolve().unwrap().node).not.toBe(original);
	expect(resolve().unwrap().node).toBe(document.getElementById("tc-field"));
});
