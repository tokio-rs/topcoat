import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

// Only these bridge imports operate entirely on JS values or scalar handles.
// Unknown imports keep memory: an import could consume a pointer into static data.
const host = /^(?:__wbg_(?:capture(?:_number|_bool)?|read(?:_number|_bool)?|write(?:_number|_bool)?|text_concat|text_empty|event_text|event_number|event_bool|event_action)_[0-9a-f]+|__wbindgen_object_(?:clone|drop)_ref)$/;

export function trimUnusedMemory(binaryen, bytes, glue) {
  const descriptor = new WebAssembly.Module(bytes);
  if (WebAssembly.Module.imports(descriptor).some(entry =>
    entry.kind !== "function" || entry.module !== "./kernel_bg.js" || !host.test(entry.name))) return bytes;
  const discarded = WebAssembly.Module.exports(descriptor).filter(entry => entry.kind !== "function");
  if (discarded.some(entry => !["memory", "__abort_handler", "__instance_terminated"].includes(entry.name))) return bytes;
  // Generated bindings must not expose memory to JS, including diagnostic helpers.
  // Match property names conservatively; an incidental match just skips trimming.
  if (discarded.some(entry => glue.includes(entry.name))) return bytes;
  const module = binaryen.readBinary(bytes);
  try {
    for (const entry of discarded) module.removeExport(entry.name);
    // Let Binaryen prove that no Wasm instruction uses memory. If memory is
    // still live, retain the original module and all its data and exports.
    module.runPasses(["remove-unused-module-elements"]);
    if (module.hasMemory()) return bytes;
    assert.equal(module.getNumDataSegments(), 0);
    assert.ok(module.validate());
    binaryen.setDebugInfo(false);
    return Buffer.from(module.emitBinary());
  } finally {
    module.dispose();
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [binaryenPath, file, glue] = process.argv.slice(2);
  const { default: binaryen } = await import(pathToFileURL(binaryenPath));
  writeFileSync(file, trimUnusedMemory(binaryen, readFileSync(file), readFileSync(glue, "utf8")));
}
