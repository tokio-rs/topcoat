// @vitest-environment happy-dom
import { expect, it } from "vitest";
import { flushEffects } from "../reactivity";
import { Runtime } from "../runtime";
import { F64 } from "../surrogate";

it("debug", async () => {
	let calls = 0;
	globalThis.fetch = (async () => { calls++; return new Response("", { status: 500 }); }) as typeof fetch;
	document.body.innerHTML = `<!--::topcoat::shard::start("/feed", "id", [])--><!--::topcoat::signal({"t":"signal","id":"s","v":1})--><!--::topcoat::dep("s")--><!--::topcoat::shard::end("id")-->`;
	const runtime = new Runtime();
	runtime.start(document);
	const shard = [...runtime.page.contentScope.children].map((c) => c.unit);
	console.log(shard.map((u) => u?.constructor.name), [...runtime.page.contentScope.children].map(c=>[...c.children].map(cc=>[...cc.dependencies])));
	runtime.context.signal("s").set(new F64(2));
	flushEffects();
	await new Promise((r) => setTimeout(r, 0));
	expect(calls).toBe(1);
});
