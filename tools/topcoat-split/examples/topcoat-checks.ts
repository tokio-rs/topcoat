import assert from "node:assert/strict";
import { Window } from "../../../crates/topcoat-runtime/browser/node_modules/happy-dom/lib/index.js";

export async function runChecks(html: string, manifest: { bundles: { name: string; entries: { id: string; source: string }[] }[] }, frames: unknown[][]) {
  assert.equal(manifest.bundles.length, 2, "page and shard should have separate bundles");
  const [page, shard] = manifest.bundles;
  assert.ok(page.entries.some(entry => shard.entries.some(other => entry.id === other.id)), "shared component expressions belong to both owners");
  const window = new Window({ url: "http://localhost/" });
  for (const name of ["window", "document", "Node", "NodeFilter", "Element", "HTMLElement", "HTMLInputElement", "HTMLSelectElement", "HTMLTextAreaElement", "Comment", "Text", "DocumentFragment", "MutationObserver", "AbortController", "AbortSignal", "Event", "CustomEvent", "DOMParser"]) {
    (globalThis as Record<string, unknown>)[name] = name === "window" ? window : (window as unknown as Record<string, unknown>)[name];
  }
  // Happy DOM decodes entities inside parsed comments, unlike browsers.
  // Restore their original bytes before exercising the real marker parser.
  const comments: string[] = [];
  const input = html.replace(/<script[^>]*>.*?<\/script>/gs, "").replace(/<!--([\s\S]*?)-->/g, (_, comment) => {
    const index = comments.push(comment) - 1;
    return `<!--fixture-comment-${index}-->`;
  });
  window.document.write(input);
  const walker = window.document.createTreeWalker(window.document, window.NodeFilter.SHOW_COMMENT);
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const comment = node as unknown as Comment;
    const match = comment.data.match(/^fixture-comment-(\d+)$/);
    if (match) comment.data = comments[Number(match[1])];
  }
  const { Runtime } = await import("../../../crates/topcoat-runtime/browser/src/runtime");
  const { flushEffects } = await import("../../../crates/topcoat-runtime/browser/src/reactivity");
  new Runtime().start(window.document as unknown as Document);
  flushEffects();
  const products = window.document.querySelectorAll(".product");
  const quantity = (index: number) => products[index].querySelector(".quantity")!.textContent;
  const total = (index: number) => products[index].querySelector(".total")!.textContent;
  assert.equal(products.length, 3);
  assert.deepEqual([quantity(0), quantity(1), quantity(2)], ["2", "1", "1"]);
  assert.equal(total(0), "21");
  assert.equal(total(2), "4.5");
  products[0].querySelector("button")!.click();
  await Promise.resolve();
  flushEffects();
  assert.deepEqual([quantity(0), quantity(1), quantity(2)], ["3", "1", "1"]);
  assert.equal(total(0), "30");
  products[2].querySelector("button")!.click();
  await Promise.resolve();
  flushEffects();
  assert.deepEqual([quantity(0), quantity(1), quantity(2)], ["3", "1", "2"]);
  assert.equal(total(2), "9");
  assert.equal(window.document.querySelector("#price")!.textContent, "10");
  products[0].querySelector(".rename")!.click();
  await Promise.resolve();
  flushEffects();
  assert.equal(products[0].querySelector("h2")!.textContent, "Coffee \u2615\u{1f680}e\u0301");
  assert.equal(products[1].querySelector("h2")!.textContent, "Cake");
  assert.equal(products[2].querySelector("h2")!.textContent, "Tea");
  assert.equal(products[0].querySelector(".empty")!.textContent, "false");
  assert.equal(products[0].querySelector(".visibility")!.hasAttribute("hidden"), false);
  products[0].querySelector(".toggle")!.click();
  await Promise.resolve();
  flushEffects();
  assert.equal(products[0].querySelector(".visibility")!.hasAttribute("hidden"), true);

  assert.throws(() => (globalThis as any).__topcoatWasm({}, "missing", []), /Missing Wasm expression/);
  const invoke = (globalThis as any).__topcoatWasm;
  const price = page.entries.find(entry => entry.source.trim() === "price.get()");
  assert.ok(price, "page has a native price expression");
  const cx = { hydrate: (value: unknown) => value };
  const description = page.entries.find(entry => entry.source.trim() === "description.get()");
  assert.ok(description, "component has a Text expression");
  const unicode = "\u{1f680}\u2615e\u0301\0<&>";
  const textSignal = { get: () => ({ dehydrate: () => ({ v: unicode }) }) };
  assert.equal(invoke(cx, description.id, [textSignal]), unicode, "Text returns the original JS string contents");
  assert.throws(() => invoke(cx, description.id, [{ get: () => ({ dehydrate: () => ({ v: "\ud800" }) }) }]), /Invalid Text/);
  assert.ok(frames.every(frame => frame.length === 0));
  const value = { dehydrate: () => ({ v: 7 }) };
  const signal = { get: () => value };
  assert.throws(() => invoke(cx, price.id, []), /arity mismatch/);
  assert.throws(() => invoke(cx, price.id, [{ get() { throw new Error("host read failed"); } }]), /host read failed/);
  assert.ok(frames.every(frame => frame.length === 0), "host errors release the capture frame");
  assert.equal(invoke(cx, price.id, [signal]), 7, "module remains callable after a host error");
  const nestedSignal = { get() {
    assert.equal(invoke(cx, price.id, [signal]), 7);
    return value;
  } };
  assert.equal(invoke(cx, price.id, [nestedSignal]), 7, "nested calls restore the outer capture frame");
  assert.ok(frames.every(frame => frame.length === 0));
  await window.happyDOM.close();
  console.log("Real Topcoat HTML hydrated; typed signals updated component and shard DOM independently, including Unicode Text.");
}
