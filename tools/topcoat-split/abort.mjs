import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

// These paths already cannot return under the generated crate's abort-only
// panic contract. Replace diagnostic construction before Wasm optimization.
// Binding misuse traps in release; debug retains wasm-bindgen's JS errors.
const [binaryenPath, file, report] = process.argv.slice(2);
const { default: binaryen } = await import(pathToFileURL(binaryenPath));
const module = binaryen.readBinary(readFileSync(file));
module.setFeatures(module.getFeatures() | binaryen.Features.BulkMemory
  | binaryen.Features.BulkMemoryOpt | binaryen.Features.NontrappingFPToInt
  | binaryen.Features.SignExt);
const rewritten = [];
for (let index = 0; index < module.getNumFunctions(); index++) {
  const ref = module.getFunctionByIndex(index);
  const info = binaryen.getFunctionInfo(ref);
  const panic = /^core\[[^\]]+\]::(?:panicking::(?:panic|assert_failed)|(?:option|result)::unwrap_failed)/.test(info.name);
  const allocation = /^alloc\[[^\]]+\]::alloc::handle_alloc_error$/.test(info.name);
  const binding = /^wasm_bindgen\[[^\]]+\]::(?:throw_str|__rt::throw_null)$/.test(info.name);
  if (!panic && !allocation && !binding) continue;
  assert.ok(!info.module, `unexpected imported abort path: ${info.name}`);
  assert.equal(info.results, binaryen.none, `abort path unexpectedly returns a value: ${info.name}`);
  binaryen.Function.setBody(ref, module.unreachable());
  rewritten.push(info.name);
}
assert.ok(module.validate());
binaryen.setDebugInfo(true);
writeFileSync(file, module.emitBinary());
writeFileSync(report, JSON.stringify(rewritten, null, 2));
module.dispose();
