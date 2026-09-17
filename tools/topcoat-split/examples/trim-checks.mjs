import assert from "node:assert/strict";
import binaryen from "binaryen";
import { trimUnusedMemory } from "../trim.mjs";

function fixture(body = '(call $read (i32.const 0))', extra = "", imported = false, name = "__wbg_read_a123") {
  const module = binaryen.parseText(`(module
    (import "./kernel_bg.js" "${name}" (func $read (param i32) (result i32)))
    ${imported ? '(import "env" "memory" (memory 1))' : '(memory 1)'}
    (export "memory" (memory 0))
    (data (i32.const 16) "live-or-dead")
    (global $handler i32 (i32.const 16))
    (export "__abort_handler" (global $handler))
    ${extra}
    (func (export "dispatch") (result i32) ${body}))`);
  module.setFeatures(binaryen.Features.BulkMemory | binaryen.Features.BulkMemoryOpt);
  assert.ok(module.validate());
  const bytes = Buffer.from(module.emitBinary());
  module.dispose();
  return bytes;
}

const input = fixture();
const trimmed = trimUnusedMemory(binaryen, input, "export function dispatch() { return wasm.dispatch(); }");
assert.ok(trimmed.length < input.length);
const exports = new WebAssembly.Instance(new WebAssembly.Module(trimmed), {
  "./kernel_bg.js": { __wbg_read_a123: () => 42 },
}).exports;
assert.equal(exports.dispatch(), 42);
assert.equal(exports.memory, undefined);
for (const body of [
  '(i32.load (i32.const 16))',
  '(memory.size)',
  '(memory.grow (i32.const 1))',
  '(block (result i32) (memory.fill (i32.const 0) (i32.const 0) (i32.const 1)) (i32.const 0))',
]) {
  const bytes = fixture(body);
  assert.deepEqual(trimUnusedMemory(binaryen, bytes, ""), bytes, "live memory must remain byte-identical");
}
for (const bytes of [
  fixture(undefined, '(func $init (drop (i32.load (i32.const 16)))) (start $init)'),
  fixture('(call_indirect (type $getter) (i32.const 0))',
    '(type $getter (func (result i32))) (table 1 funcref) (elem (i32.const 0) $live) (func $live (type $getter) (i32.load (i32.const 16)))'),
  fixture('(block (result i32) (memory.init $passive (i32.const 0) (i32.const 0) (i32.const 3)) (i32.const 0))',
    '(data $passive "abc")'),
  fixture(undefined, "", true),
  fixture(undefined, '(export "user_pointer" (global $handler))'),
  fixture(undefined, "", false, "unknown_pointer_consumer"),
]) assert.deepEqual(trimUnusedMemory(binaryen, bytes, ""), bytes);
for (const glue of ["wasm.memory", 'wasm["memory"]', "wasm.__abort_handler"]) {
  assert.deepEqual(trimUnusedMemory(binaryen, input, glue), input, "JS-visible memory must remain");
}
console.log("Unused-memory checks passed: live loads, growth, bulk operations, imports, exports and glue are preserved.");
