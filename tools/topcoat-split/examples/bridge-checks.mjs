import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { buildClient } from "../build-client.mjs";

const tool = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = resolve(tool, "../../target/typed-bridge-checks");
const client = process.env.TOPCOAT_SPLIT_CLIENT_TOOLCHAIN ?? "stable";
const wasmOpt = process.env.WASM_OPT ?? "wasm-opt";
const esbuild = process.env.ESBUILD ?? "esbuild";
const kinds = ["Text", "bool", "()", "f64", "i8", "i16", "i32", "u8", "u16", "u32"];
const samples = [["", "\u2615\u{1f680}e\u0301\0"], [false, true], [null], [-Number.MAX_VALUE, -0, 0.5, Number.MAX_VALUE],
  [-128, 127], [-32768, 32767], [-2147483648, 2147483647], [0, 255], [0, 65535], [0, 4294967295]];
mkdirSync(join(output, "src"), { recursive: true });
const cargo = readFileSync(join(tool, "src/wasm.rs"), "utf8").match(/pub const CARGO: &str = r#"([\s\S]*?)"#;/)[1];
writeFileSync(join(output, "Cargo.toml"), `[workspace]\n[package]\nname = "typed-bridge-checks"\nversion = "0.0.0"\nedition = "2024"\n${cargo}`);
writeFileSync(join(output, "host.js"), readFileSync(join(tool, "src/wasm/host.js")));
const sampleSource = `[${samples.map(values =>
  `[${values.map(value => Object.is(value, -0) ? "-0" : JSON.stringify(value)).join(",")}]`).join(",\n")}]`;
const arms = kinds.map((kind, index) => {
  const type = kind === "Text" ? "__topcoat::Text" : kind;
  return `${index} => {
    let signal = __topcoat::Signal::<${type}>::new(0);
    match operation {
      0 => <${type} as ClientValue>::capture(0).into_js(),
      1 => signal.get().into_js(),
      2 => { signal.set(<${type} as ClientValue>::capture(1)); signal.get().into_js() },
      ${index >= 3 ? "4 => { signal.increment(); signal.get().into_js() }, 5 => { signal.decrement(); signal.get().into_js() }," : ""}
      _ => core::arch::wasm32::unreachable(),
    }
  }`;
}).join(",\n");
writeFileSync(join(output, "src/lib.rs"), `#![no_std]
#![forbid(unsafe_code)]
#![allow(dead_code)]
extern crate alloc;
${readFileSync(join(tool, "src/wasm/allocator.rs.txt"), "utf8")}\n${readFileSync(join(tool, "src/wasm/runtime.rs.txt"), "utf8")}
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn dispatch(kind: u32, operation: u32) -> wasm_bindgen::JsValue {
  use __topcoat::ClientValue;
  if operation == 3 {
    __topcoat::Signal::<f64>::new(0).set(f64::INFINITY);
    return wasm_bindgen::JsValue::NULL;
  }
  match kind { ${arms}, _ => core::arch::wasm32::unreachable() }
}
`);
for (const profile of ["debug", "release"]) {
  const pkg = join(output, profile);
  const { wasm, glue, allocator } = buildClient({ directory: output, targetDir: join(output, "target"), pkg,
    name: "typed_bridge_checks", profile, client, wasmOpt });
  assert.doesNotMatch(glue, /__wbindgen_number_get|__wbindgen_boolean_get/, "primitive inputs must use typed imports");
  const module = new WebAssembly.Module(readFileSync(wasm));
  if (profile === "release") {
    assert.equal(WebAssembly.Module.customSections(module, "name").length, 0);
    assert.equal(WebAssembly.Module.customSections(module, "producers").length, 0);
    assert.ok(WebAssembly.Module.imports(module).every(entry => entry.module.length === 1 && entry.name.length === 1));
    assert.doesNotMatch(glue, /TextEncoder|TextDecoder|JSON\.(parse|stringify)/);
  } else {
    assert.ok(WebAssembly.Module.customSections(module, "name").length > 0);
    assert.ok(WebAssembly.Module.exports(module).some(entry => entry.name === "dispatch"));
  }
  const host = glue.match(/from ['"]([^'"]+\/host\.js)['"]/)[1];
  const runner = `import assert from "node:assert/strict";
import {readFileSync} from "node:fs";
import init, {dispatch} from ${JSON.stringify(join(pkg, "kernel.js"))};
import {frames} from ${JSON.stringify(join(pkg, host))};
await init({module_or_path:readFileSync(${JSON.stringify(wasm)})${allocator ? `,allocator_module:readFileSync(${JSON.stringify(allocator.file)})` : ""}});
const kinds = ${JSON.stringify(kinds)};
// These values cannot be JSON encoded without losing NaN, infinity, and -0.
const samples = ${sampleSource};
const invalid = [[42, "\\ud800"], [0, "false"], [undefined], [NaN, Infinity, "1"], [-129, 128], [-32769, 32768], [-2147483649, 2147483648], [-1, 256], [-1, 65536], [-1, 4294967296, 1.5]];
const cx = {hydrate: value => ({dehydrate: () => value})};
for (const [kind, bridge] of kinds.entries()) {
  let value;
  const signal = {dehydrate: () => ({v:value}), get: () => ({dehydrate: () => ({v:value})}), set: next => { value = next.dehydrate().v; }};
  frames.push({cx, captures:[signal,signal], entry:{captures:[{bridge},{bridge}]}});
  try {
    for (value of samples[kind]) {
      for (const operation of [0, 1, 2]) assert.equal(dispatch(kind, operation), value, bridge);
    }
    for (value of invalid[kind]) {
      for (const operation of [0, 1, 2]) assert.throws(() => dispatch(kind, operation), TypeError, bridge);
    }
    value = samples[kind][0];
    if (bridge === "f64") {
      assert.throws(() => dispatch(kind, 3), TypeError, "invalid Wasm writes are rejected");
      assert.equal(value, samples[kind][0], "invalid writes leave the signal unchanged");
    }
    assert.equal(dispatch(kind, 1), value, "valid calls work after failed validation");
    if (kind >= 3) {
      value = 2;
      assert.equal(dispatch(kind, 4), 3, "increment uses the signal value");
      assert.equal(dispatch(kind, 5), 2, "decrement uses the signal value");
    }
    if (kind >= 4) {
      value = samples[kind][1];
      if (${JSON.stringify(profile)} === "release") assert.equal(dispatch(kind, 4), samples[kind][0], "release overflow wraps");
      else {
        assert.throws(() => dispatch(kind, 4), WebAssembly.RuntimeError, "debug overflow traps");
        assert.equal(value, samples[kind][1], "overflow does not write the signal");
      }
    }
  } finally { frames.pop(); }
}
console.log(${JSON.stringify(`${profile}: typed captures, reads, writes, bounds, and invalid input checks passed.`)});
`;
  writeFileSync(join(pkg, "checks.mjs"), runner);
  execFileSync(esbuild, [join(pkg, "checks.mjs"), "--bundle", "--platform=node", "--format=esm", `--outfile=${join(pkg, "run.mjs")}`], { stdio: "inherit" });
  execFileSync(process.execPath, [join(pkg, "run.mjs")], { stdio: "inherit" });
}
