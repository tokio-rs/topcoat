import assert from "node:assert/strict";
import { Window } from "../../../crates/topcoat-runtime/browser/node_modules/happy-dom/lib/index.js";
import { Runtime } from "../../../crates/topcoat-runtime/browser/src/runtime";
import { Event as RuntimeEvent } from "../../../crates/topcoat-runtime/browser/src/surrogate/event";
import { flushEffects } from "../../../crates/topcoat-runtime/browser/src/reactivity";

type Entry = {id: string; source: string; async: boolean; captures: {name: string}[]};
export async function runChecks(manifest: {bundles: {name: string; entries: Entry[]}[]}, frames: unknown[][]) {
  assert.deepEqual(manifest.bundles.map(b => b.entries.length).sort(), [5, 7]);
  const base = process.env.COFFEE_URL ?? "http://127.0.0.1:8083";
  const network = globalThis.fetch;
  const NativeAbortController = globalThis.AbortController;
  globalThis.fetch = async (input, init) => {
    const controller = new NativeAbortController();
    const abort = () => controller.abort();
    if (init?.signal?.aborted) abort();
    else init?.signal?.addEventListener("abort", abort, {once:true});
    try { return await network(new URL(String(input), base), {...init, signal: controller.signal}); }
    finally { init?.signal?.removeEventListener("abort", abort); }
  };
  const window = new Window({url: base + "/menu"});
  for (const name of ["window", "document", "Node", "NodeFilter", "Element", "HTMLElement", "HTMLInputElement", "HTMLSelectElement", "HTMLTextAreaElement", "Comment", "Text", "DocumentFragment", "MutationObserver", "AbortController", "AbortSignal", "Event", "CustomEvent", "DOMParser", "history", "location"]) {
    (globalThis as any)[name] = name === "window" ? window : (window as any)[name];
  }
  async function load(path: string) {
    const response = await fetch(path);
    assert.equal(response.status, 200);
    let html = await response.text();
    // Apply the completed streamed regions before hydration. Happy DOM does
    // not execute the inline parser-time swap scripts like a real browser.
    const swaps: [string, string][] = [];
    html = html.replace(/<template data-topcoat-swap="(\d+)">([\s\S]*?)<\/template>/g, (_, id, content) => { swaps.push([id, content]); return ""; });
    for (const [id, content] of swaps) {
      const open = `<!--topcoat::region::start(${id})-->`;
      const close = `<!--topcoat::region::end(${id})-->`;
      const start = html.indexOf(open), end = html.indexOf(close, start);
      assert.ok(start >= 0 && end >= start);
      html = html.slice(0, start + open.length) + content + html.slice(end);
    }
    const comments: string[] = [];
    const input = html.replace(/<script[^>]*>.*?<\/script>/gs, "").replace(/<!--([\s\S]*?)-->/g, (_,comment) => `<!--fixture-comment-${comments.push(comment)-1}-->`);
    window.document.open();
    window.document.write(input);
    const walker = window.document.createTreeWalker(window.document, window.NodeFilter.SHOW_COMMENT);
    for (let node=walker.nextNode(); node; node=walker.nextNode()) {
      const comment = node as any;
      const match = comment.data.match(/^fixture-comment-(\d+)$/);
      if(match) comment.data=comments[Number(match[1])];
    }
    const runtime = new Runtime();
    runtime.start(window.document as any);
    flushEffects();
    return runtime;
  }
  async function until(test: () => boolean) {
    for(let i=0;i<150;i++) {
      flushEffects();
      if(test()) return;
      await new Promise(resolve => setTimeout(resolve,20));
    }
    assert.ok(test(), "DOM did not reach expected state: " + window.document.body.textContent);
  }
  const menu = await load("/menu");
  const input = window.document.querySelector('input[type="search"]')! as any;
  const clear = [...window.document.querySelectorAll("button")].find(b => b.textContent === "Clear")!;
  assert.ok(clear.disabled);
  input.value = "no matching coffee \u{1f680}e\u0301";
  input.dispatchEvent(new window.Event("input", {bubbles:true}));
  await until(() => window.document.body.textContent.includes("Nothing matches."));
  assert.doesNotMatch(clear.outerHTML, /\sdisabled(?:=|\s|>)/);
  clear.dispatchEvent(new window.Event("click", {bubbles:true}));
  await until(() => input.value === "" && !window.document.body.textContent.includes("Nothing matches."));
  assert.ok(clear.disabled);
  const drinkLink = [...window.document.querySelectorAll('a[href^="/menu/"]')][0]!;
  assert.ok(drinkLink);
  const path = drinkLink.getAttribute("href")!;
  menu.page.contentScope.dispose();
  const drink = await load(path);
  const button = (label: string) => [...window.document.querySelectorAll("button")].find(b => b.textContent.trim() === label)!;
  button("+").dispatchEvent(new window.Event("click", {bubbles:true}));
  flushEffects();
  assert.equal(window.document.querySelector("p.w-8")!.textContent,"2");
  button("-").dispatchEvent(new window.Event("click", {bubbles:true})); button("-").dispatchEvent(new window.Event("click", {bubbles:true}));
  flushEffects();
  assert.equal(window.document.querySelector("p.w-8")!.textContent,"1");
  button("Order").dispatchEvent(new window.Event("click", {bubbles:true}));
  await until(() => window.document.body.textContent.includes("Coming right up: 1 x"));
  const confirmation = [...window.document.querySelectorAll("p")].find(p => p.textContent.includes("Coming right up:"))!;
  assert.ok(confirmation);
  // Happy DOM conflates colon-suffixed binding names with ordinary names in
  // some attribute getters. Inspect exact names, not its [hidden] selector.
  assert.equal([...confirmation.attributes].some(attr => attr.name === "hidden"), false);

  // Exercise real Wasm async dispatch with controlled responses. Two handlers
  // must retain different captures while both are suspended in the same module.
  const order = manifest.bundles.flatMap(b=>b.entries).find(e=>e.async)!;
  const invoke = (globalThis as any).__topcoatWasm;
  const pending: {resolve:(value:unknown)=>void; reject:(error:unknown)=>void}[]=[];
  const cx = { hydrate: (value:any):any => value?.t === "Procedure" ? {call: () => new Promise((resolve,reject)=>pending.push({resolve,reject}))} : {dehydrate:()=>value} };
  function signal(value: unknown) {
    let current = {t:"Wasm", v:value};
    return {get:()=>({dehydrate:()=>current}),set:(value:any)=>{current=value.dehydrate();},value:()=>current.v};
  }
  const first=signal(""), second=signal("");
  const start = (confirmation: unknown, name: string) => invoke(cx, order.id, order.captures.map(c => ({confirmation, name:{dehydrate:()=>({v:name})}, quantity:signal(2)} as any)[c.name]))(new RuntimeEvent(new window.Event("click") as any));
  const a=start(first,"A"), b=start(second,"B");
  assert.equal(pending.length,2,"handlers run synchronously up to first await");
  assert.ok(frames.every(f=>f.length===0));
  pending[1].resolve({dehydrate:()=>"B \u2615\u{1f680}"});
  await until(() => second.value() !== "");
  await b;
  assert.equal(first.value(),""); assert.equal(second.value(),"B \u2615\u{1f680}");
  pending[0].reject(new Error("order failed"));
  await assert.rejects(a,/order failed/);
  assert.equal(first.value(),"");
  assert.ok(frames.every(f=>f.length===0));
  const c=start(first,"retry");
  pending[2].resolve({dehydrate:()=>"retry succeeded"});
  await c;
  assert.equal(first.value(),"retry succeeded");
  assert.ok(frames.every(f=>f.length===0));
  const malformed = start(first,"bad response");
  pending[3].resolve({dehydrate:()=>42});
  await assert.rejects(malformed, /Invalid Text/);
  assert.equal(first.value(),"retry succeeded", "invalid response must not change state");
  assert.ok(frames.every(f=>f.length===0));
  drink.page.contentScope.dispose();
  await window.happyDOM.close();
  globalThis.fetch=network;
  console.log("Coffee-shop Wasm checks passed: Text search/shards, quantity, HTTP ordering, Unicode, concurrent handlers, rejection and retry.");
}
