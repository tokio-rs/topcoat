import assert from "node:assert/strict";
import { install } from "../src/wasm/bridge.js";

export async function runChecks(kernel, host, memory, release = false) {
  const handles = [];
  const active = new Set();
  install([{
    ...kernel, frames: host.frames,
    dispatch(index) { const id = kernel.dispatch(index); assert.ok(!active.has(id)); active.add(id); handles.push(id); return id; },
    cancel(id) { kernel.cancel(id); assert.ok(active.delete(id)); },
    entries: Array.from({length:8}, (_,index) => ({id:String(index),index,async:true,handler:true,result:"()",captures:[{bridge:(index===4 || index===5)?"f64":"Text"},{bridge:"Text"}]})),
    procedures: [{index:0,id:"text",args:["Text"],result:"Text"},{index:1,id:"number",args:[],result:"f64"}],
  }]);
  const pending = [];
  const cx = {hydrate(value) {
    if (value?.t === "Procedure") return {call: (...args) => new Promise((resolve,reject)=>pending.push({resolve,reject,args:args.map(arg=>arg.dehydrate()),id:value.id}))};
    return {dehydrate:()=>value};
  }};
  const signal = (value = "", onWrite = () => {}) => ({
    get:()=>({dehydrate:()=>({v:value})}),
    set(next) { value=next.dehydrate().v; onWrite(); },
    value:()=>value,
  });
  let prevented = 0;
  const invoke = (index, target, prefix="") => globalThis.__topcoatWasm(cx,String(index),[target,{dehydrate:()=>({v:prefix})}])({prevent_default(){prevented++;}});
  const settle = async () => { for(let i=0;i<8;i++) await Promise.resolve(); };
  const result = value => ({dehydrate:()=>value});
  const first = signal(), second = signal();
  const a = invoke(0, first, "A"), b = invoke(0, second, "B");
  assert.equal(prevented, 2, "first poll runs synchronously");
  assert.equal(pending.length, 2);
  assert.notEqual(handles[0], handles[1]);
  assert.deepEqual(host.frames, []);
  pending[1].resolve(result("coffee"));
  await settle();
  assert.deepEqual(pending[2].args, ["coffee"], "second await uses a Rust local from the first await");
  pending[0].resolve(result("tea"));
  await settle();
  assert.deepEqual(pending[3].args, ["tea"]);
  pending[2].resolve(result("\u2615\u{1f680}"));
  await b;
  assert.equal(second.value(), "Bcoffee\u2615\u{1f680}");
  assert.equal(first.value(), "");
  pending[3].resolve(result("!"));
  await a;
  assert.equal(first.value(), "Atea!");
  assert.equal(host.drops, 2, "completed futures are dropped");

  // Rejections, including falsy reasons, drop the future without executing
  // the continuation. A subsequent task must not inherit the old capture environment.
  for (const reason of [new Error("failed"), false, 0, "", undefined]) {
    const index = pending.length, drops = host.drops;
    const task = invoke(0, first, "rejected");
    const checked = task.then(()=>assert.fail("must reject"), error=>reason === undefined ? assert.match(error.message, /Procedure rejected/) : assert.equal(error, reason));
    pending[index].reject(reason);
    await checked;
    assert.equal(first.value(), "Atea!");
    assert.equal(host.drops, drops + 1);
  }
  const drops = host.drops;
  await invoke(1, first, "immediate");
  assert.equal(first.value(), "immediate");
  assert.equal(host.drops, drops+1);
  await assert.rejects(invoke(2, first), /without a Topcoat procedure/);
  assert.equal(host.drops, drops+2, "unsupported suspension drops its future");

  const requests = pending.length;
  await invoke(7, first, "unpolled");
  assert.equal(pending.length, requests, "dropping an unpolled procedure does not start a request");
  assert.equal(host.drops, drops+3);

  // A concurrent wait is rejected. A late response from the abandoned
  // operation must not resume a new handler.
  const abandoned = pending.length;
  await assert.rejects(invoke(3, first), /Concurrent procedure waits/);
  const retryIndex = pending.length;
  const retry = invoke(0, second, "retry");
  pending[abandoned].resolve(result("stale"));
  await settle();
  assert.equal(pending.length, retryIndex+1);
  pending[retryIndex].resolve(result("one"));
  await settle();
  pending[retryIndex+1].resolve(result("two"));
  await retry;
  assert.equal(second.value(), "retryonetwo");

  // Signal writes can synchronously enter a second handler while the first
  // future is still being polled, with a separate task and capture environment.
  let nested;
  const target = signal("", ()=>{nested=invoke(1, second, "nested");});
  await invoke(1, target, "outer");
  await nested;
  assert.equal(target.value(), "outer");
  assert.equal(second.value(), "nested");
  assert.notEqual(handles.at(-1), handles.at(-2));

  const number = signal(0);
  const numericIndex = pending.length;
  const numeric = invoke(4, number);
  pending[numericIndex].resolve(result(1.25));
  await numeric;
  assert.equal(number.value(),1.25);
  const invalidIndex = pending.length;
  const invalid = invoke(4, number);
  pending[invalidIndex].resolve(result("wrong type"));
  await assert.rejects(invalid,/Invalid f64/);
  assert.equal(number.value(),1.25);
  // Warm up the allocator, then repeatedly allocate and free both immediately
  // completed and suspended futures. Linear memory must remain bounded.
  async function batch() {
    for (let i=0; i<32; i++) await invoke(1, first, "reuse");
    const start = pending.length;
    const tasks = Array.from({length:32}, (_,i) => invoke(i % 2 ? 4 : 5, number));
    const complete = Promise.allSettled(tasks);
    for (let i=0; i<32; i++) {
      if (i % 2) pending[start+i].reject(new Error("expected"));
      else pending[start+i].resolve(result(i));
    }
    const outcomes = await complete;
    assert.equal(outcomes.filter(item=>item.status==="fulfilled").length,16);
    assert.equal(outcomes.filter(item=>item.status==="rejected").length,16);
  }
  await batch();
  const memoryAfterWarmup = memory.buffer.byteLength;
  for (let i=0; i<100; i++) await batch();
  assert.equal(memory.buffer.byteLength, memoryAfterWarmup, "wee_alloc must reuse freed task memory");
  assert.equal(kernel.invalid_number(2),2);
  assert.throws(()=>kernel.invalid_number("wrong"),WebAssembly.RuntimeError,"bridge preconditions abort directly");
  assert.equal(kernel.bounds(1),2);
  assert.throws(()=>kernel.bounds(2),WebAssembly.RuntimeError,"compiler-inserted bounds failures abort");
  assert.deepEqual(host.frames, []);
  assert.equal(host.drops, handles.length, "every started future was dropped");
  assert.equal(active.size, 0, "completed and rejected tasks release their handles");
  assert.ok(handles.every(task => task.__wbg_ptr === 0), "Rust task handles are explicitly freed");
  return async (reenter = false) => {
    if (reenter) {
      const target = signal("", () => kernel.resume(handles.at(-1)));
      const failure = release ? WebAssembly.RuntimeError : /recursive use/;
      await assert.rejects(invoke(1, target, "exclusive"), failure,
        "the same task cannot be polled through a second mutable borrow");
      const count = handles.length;
      await assert.rejects(invoke(1, first), failure,
        "a binding failure disables the instance because Rust cannot unwind it");
      assert.equal(handles.length, count);
      assert.deepEqual(host.frames, []);
      return;
    }
    const value = first.value();
    await assert.rejects(invoke(6, first), WebAssembly.RuntimeError, "async abort rejects the JS promise");
    const count = handles.length;
    await assert.rejects(invoke(1, first, "must not run"), WebAssembly.RuntimeError, "an aborted module cannot run another handler");
    assert.equal(first.value(), value);
    assert.equal(handles.length, count, "the aborted module was not entered again");
    assert.deepEqual(host.frames, []);
  };
}
