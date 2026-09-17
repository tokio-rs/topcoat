import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

// Operate after wasm-bindgen and before optimization can inline the allocator.
// The pinned wee_alloc ABI is checked here rather than inferred from call sites.
const [binaryenPath, mode, file, metadata] = process.argv.slice(2);
const { default: binaryen } = await import(pathToFileURL(binaryenPath));
const module = binaryen.readBinary(readFileSync(file));
module.setFeatures(module.getFeatures() | binaryen.Features.BulkMemory
  | binaryen.Features.BulkMemoryOpt | binaryen.Features.NontrappingFPToInt
  | binaryen.Features.SignExt);
if (mode === "memory") {
  module.addMemoryImport("0", "topcoat_allocator", "__topcoat_memory");
  assert.ok(module.validate());
  binaryen.setDebugInfo(metadata === "debug");
  writeFileSync(file, module.emitBinary());
  module.dispose();
  process.exit(0);
}
assert.equal(typeof module.getDataSegmentInfo, "function", "Binaryen 132 JS API is required");
for (let index = 0; index < module.getNumDataSegments(); index++) {
  const segment = module.getDataSegmentInfo(module.getDataSegmentByIndex(index));
  assert.ok(!segment.passive, "passive client data segments are unsupported");
  if (mode === "allocator") assert.ok(segment.offset + segment.data.byteLength <= 65536);
  else assert.ok(segment.offset >= 65536, "page data overlaps allocator reservation");
}
const stack = module.getGlobal("__stack_pointer");
if (stack) {
  const init = binaryen.getExpressionInfo(binaryen.getGlobalInfo(stack).init);
  assert.equal(init.id, binaryen.ConstId);
  if (mode === "allocator") assert.ok(init.value >= 16384 && init.value <= 65536);
  else assert.ok(init.value - 1048576 >= 65536, "page stack overlaps allocator reservation");
}
const methods = new Map();
for (let index = 0; index < module.getNumFunctions(); index++) {
  const info = binaryen.getFunctionInfo(module.getFunctionByIndex(index));
  if (!info.name.includes("wee_alloc[") || !info.name.includes("GlobalAlloc")) continue;
  const operation = />::alloc$/.test(info.name) ? "a" : />::dealloc$/.test(info.name) ? "d" : null;
  if (/>::(realloc|alloc_zeroed)$/.test(info.name)) continue;
  assert.ok(operation, `unsupported wee_alloc method: ${info.name}`);
  assert.ok(!methods.has(operation), `duplicate wee_alloc method: ${operation}`);
  assert.deepEqual(binaryen.expandType(info.params), Array(operation === "a" ? 3 : 4).fill(binaryen.i32));
  assert.equal(info.results, operation === "a" ? binaryen.i32 : binaryen.none);
  methods.set(operation, info);
}
if (mode === "allocator") {
  assert.equal(methods.size, 2);
  const address = binaryen.getExportInfo(module.getExport("allocator_address")).value;
  while (module.getNumExports()) module.removeExport(binaryen.getExportInfo(module.getExportByIndex(0)).name);
  for (const [operation, info] of methods) {
    const operands = binaryen.expandType(info.params).map((type, index) =>
      index === 0 ? module.call(address, [], binaryen.i32) : module.local.get(index, type));
    module.addFunction(operation, info.params, info.results, [], module.call(info.name, operands, info.results));
    module.addFunctionExport(operation, operation);
  }
  // The allocator's data and stack fit in the first 64 KiB. Page data and
  // its separate stack start after that reservation.
  assert.ok(module.getMemoryInfo().initial <= 1, "allocator exceeds its reserved memory page");
  module.addMemoryImport("0", "env", "memory");
  module.runPasses(["remove-unused-module-elements"]);
} else {
  assert.equal(mode, "client");
  for (const [operation, info] of methods) {
    module.removeFunction(info.name);
    module.addFunctionImport(info.name, "topcoat_allocator", operation === "a" ? "__topcoat_allocate" : "__topcoat_deallocate", info.params, info.results);
  }
  // Allocator policy vtables occupy table slots even after their callers are
  // removed. Preserve page function indices while replacing those private slots.
  module.addFunction("allocator_private_slot", binaryen.none, binaryen.none, [], module.unreachable());
  const segments = Array.from({length: module.getNumElementSegments()}, (_, index) =>
    binaryen.getElementSegmentInfo(module.getElementSegmentByIndex(index)));
  for (const segment of segments) {
    const data = segment.data.map(name => name.includes("wee_alloc[") ? "allocator_private_slot" : name);
    module.removeElementSegment(segment.name);
    module.addActiveElementSegment(segment.table, segment.name, data, segment.offset);
  }
  module.runPasses(["remove-unused-module-elements"]);
  // Reject unexpected retained allocator helpers after redirecting its entry points.
  for (let index = 0; index < module.getNumFunctions(); index++) {
    const info = binaryen.getFunctionInfo(module.getFunctionByIndex(index));
    assert.ok(!info.name.includes("wee_alloc[") || info.module === "topcoat_allocator"
      || /GlobalAlloc>::(realloc|alloc_zeroed)$/.test(info.name),
      `allocator implementation remained in client: ${info.name}`);
  }
  if (methods.size) {
    const memory = module.getMemoryInfo();
    assert.ok(!memory.shared && !memory.is64 && memory.max === undefined);
    writeFileSync(metadata, JSON.stringify({ initial: memory.initial }));
  } else {
    writeFileSync(metadata, "null");
  }
}
assert.ok(module.validate());
// Preserve names until the normal release optimization and glue rename steps.
binaryen.setDebugInfo(true);
writeFileSync(file, module.emitBinary());
module.dispose();
