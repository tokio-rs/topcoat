import assert from "node:assert/strict";
import binaryen from "binaryen";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync, cpSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { buildClient } from "../build-client.mjs";

const tool = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = resolve(tool, "../../target/async-bridge-checks");
mkdirSync(join(output, "src"), { recursive: true });
const cargo = readFileSync(join(tool, "src/wasm.rs"), "utf8").match(/pub const CARGO: &str = r#"([\s\S]*?)"#;/)[1];
writeFileSync(join(output, "Cargo.toml"), `[workspace]\n[package]\nname = "async-bridge-checks"\nversion = "0.0.0"\nedition = "2024"\n${cargo}`);
writeFileSync(join(output, "host.js"), readFileSync(join(tool, "src/wasm/host.js"), "utf8") + `
export let drops = 0;
export function record_drop() { drops++; }
`);
const runtime = readFileSync(join(tool, "src/wasm/runtime.rs.txt"), "utf8")
  .replace("// ASYNC_RUNTIME", readFileSync(join(tool, "src/wasm/async.rs.txt"), "utf8"));
writeFileSync(join(output, "src/lib.rs"), `#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;
${readFileSync(join(tool, "src/wasm/allocator.rs.txt"), "utf8")}
${runtime}
use __topcoat::{Arguments, ClientValue, Signal, Text, procedure};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(module = "/host.js")]
extern "C" {
    fn record_drop();
    fn event_action(action: u32);
}
struct Guard;
impl Drop for Guard {
    fn drop(&mut self) { record_drop(); }
}
fn request(value: Text) -> impl core::future::Future<Output = Text> {
    let args = Arguments::new();
    args.push(&value.into_js());
    procedure(0, args)
}
#[repr(align(256))]
struct Aligned([u8; 513]);
#[wasm_bindgen]
pub fn allocation_check(value: u8) -> bool {
    let block = core::hint::black_box(alloc::boxed::Box::new(Aligned([value; 513])));
    let address = &*block as *const Aligned as usize;
    let mut bytes = alloc::vec![value; 257];
    bytes.reserve(65536);
    bytes.resize(65536, value);
    address % 256 == 0 && block.0.iter().all(|byte| *byte == value)
        && core::hint::black_box(bytes).iter().all(|byte| *byte == value)
}
#[wasm_bindgen]
pub fn invalid_number(value: JsValue) -> f64 { f64::from_js(value) }
#[wasm_bindgen]
pub fn bounds(index: usize) -> u8 { [1, 2][index] }
#[wasm_bindgen]
pub fn dispatch(index: u32) -> JsValue {
    // A distinct future type exercises a second allocation size in the pool.
    if index == 1 {
        return __topcoat::run(async {
            let _guard = Guard;
            Signal::<Text>::new(0).set(Text::capture(1));
            JsValue::NULL
        });
    }
    if index == 5 {
        return __topcoat::run(async {
            let _guard = Guard;
            let state = core::hint::black_box([0_u8; 1024]);
            let value: f64 = procedure(1, Arguments::new()).await;
            let adjustment = core::hint::black_box(&state)[0] as f64;
            Signal::<f64>::new(0).set(value + adjustment);
            JsValue::NULL
        });
    }
    __topcoat::run(async move {
        let _guard = Guard;
        let signal = Signal::<Text>::new(0);
        let prefix = Text::capture(1);
        match index {
            0 => {
                event_action(0);
                let first = request(prefix.clone()).await;
                let second = request(first.clone()).await;
                signal.set(prefix.concat(&first).concat(&second));
            }
            1 => signal.set(prefix),
            2 => core::future::pending::<()>().await,
            3 => {
                // This requires a general executor and must reject explicitly.
                let mut a = core::pin::pin!(request(prefix.clone()));
                let mut b = core::pin::pin!(request(prefix));
                core::future::poll_fn(|cx| {
                    let _ = a.as_mut().poll(cx);
                    let _ = b.as_mut().poll(cx);
                    core::task::Poll::<()>::Pending
                }).await;
            }
            4 => {
                let value: f64 = procedure(1, Arguments::new()).await;
                Signal::<f64>::new(0).set(value);
            }
            7 => drop(request(prefix)),
            _ => core::arch::wasm32::unreachable(),
        }
        JsValue::NULL
    })
}
`);
for (const profile of ["debug", "release"]) {
  const pkg = join(output, profile);
  const { wasm, glue, allocator } = buildClient({directory: output, targetDir: join(output, "target"), pkg,
    name: "async_bridge_checks", profile, client: process.env.TOPCOAT_SPLIT_CLIENT_TOOLCHAIN ?? "stable",
    wasmOpt: process.env.WASM_OPT ?? "wasm-opt"});
  assert.doesNotMatch(glue, /makeMutClosure|queueMicrotask/, "no generic future executor or Rust Promise callbacks");
  if (profile === "release") assert.doesNotMatch(glue, /TextEncoder|JSON\.(parse|stringify)/, "no release text encoding or serialization glue");
  const host = glue.match(/from ['"]([^'"]+\/host\.js)['"]/)[1];
  assert.ok(allocator, "the async fixture must use the external allocator");
  const second = join(output, `${profile}-second`);
  cpSync(pkg, second, {recursive:true});
  const stale = join(output, `${profile}-stale`);
  cpSync(pkg, stale, {recursive:true});
  const oom = join(output, `${profile}-oom`);
  cpSync(pkg, oom, {recursive:true});
  const allocatorModule = binaryen.readBinary(readFileSync(allocator.file));
  const allocate = binaryen.getExportInfo(allocatorModule.getExport("a")).value;
  binaryen.Function.setBody(allocatorModule.getFunction(allocate), allocatorModule.i32.const(0));
  assert.ok(allocatorModule.validate());
  writeFileSync(join(oom, "oom.wasm"), allocatorModule.emitBinary());
  allocatorModule.dispose();
  const runner = `import {readFileSync} from "node:fs";
import init, * as kernel from ${JSON.stringify(join(pkg, "kernel.js"))};
import * as host from ${JSON.stringify(join(pkg, host))};
import initSecond, * as secondKernel from ${JSON.stringify(join(second, "kernel.js"))};
import initStale, * as staleKernel from ${JSON.stringify(join(stale, "kernel.js"))};
import initOom, * as oomKernel from ${JSON.stringify(join(oom, "kernel.js"))};
import * as secondHost from ${JSON.stringify(join(second, host))};
import {compileAllocator} from ${JSON.stringify(join(tool, "allocator/loader.js"))};
import assert from "node:assert/strict";
import {runChecks} from ${JSON.stringify(join(tool, "examples/async-checks-runner.mjs"))};
const bytes = readFileSync(${JSON.stringify(wasm)});
const allocatorBytes = readFileSync(${JSON.stringify(allocator.file)});
// Simulate two browser bundles requesting the same content-hashed URL.
const originalFetch = globalThis.fetch;
let fetches = 0;
globalThis.fetch = async () => { fetches++; return new Response(allocatorBytes); };
let compiledAllocator;
try {
  const url = new URL("https://allocator.test/allocator.wasm");
  const modules = await Promise.all([compileAllocator(url), compileAllocator(url.href)]);
  assert.equal(fetches, 1);
  assert.equal(modules[0], modules[1]);
  compiledAllocator = modules[0];
  let attempts = 0;
  globalThis.fetch = async () => ++attempts === 1
    ? new Response("unavailable", {status:503}) : new Response(allocatorBytes);
  await assert.rejects(compileAllocator("https://allocator.test/retry.wasm"), /503/);
  assert.ok(await compileAllocator("https://allocator.test/retry.wasm") instanceof WebAssembly.Module);
  assert.equal(attempts, 2, "failed allocator downloads can be retried");
} finally { globalThis.fetch = originalFetch; }
assert.equal(await compileAllocator(compiledAllocator), compiledAllocator);
const bytesModule = await compileAllocator(allocatorBytes);
assert.equal(await compileAllocator(allocatorBytes), bytesModule);
const exports = await init({module_or_path:bytes, allocator_module: compiledAllocator});
const other = secondKernel.initSync({module:bytes, allocator_module: compiledAllocator});
assert.equal(await initSecond({module_or_path:bytes, allocator_module: compiledAllocator}), other);
const memoryName = WebAssembly.Module.exports(new WebAssembly.Module(bytes)).find(item => item.kind === "memory").name;
assert.notEqual(exports[memoryName], other[memoryName]);
const foreign = secondKernel.dispatch(2);
assert.throws(() => kernel.cancel(foreign), /expected instance of/);
assert.notEqual(foreign.__wbg_ptr, 0, "foreign task rejection must not consume it");
secondKernel.cancel(foreign);
assert.throws(() => kernel.cancel({free() { assert.fail("must not call an arbitrary free method"); }}), /expected instance of/);
const otherSize = other[memoryName].buffer.byteLength;
for (const value of [0, 47, 255]) assert.ok(kernel.allocation_check(value));
assert.equal(other[memoryName].buffer.byteLength, otherSize, "growing one heap leaves the other unchanged");
for (const value of [0, 11, 255]) assert.ok(secondKernel.allocation_check(value));
const decode = TextDecoder.prototype.decode;
let decodes = 0, checkAbort;
TextDecoder.prototype.decode = () => { decodes++; throw new Error("unexpected Wasm string decoding"); };
try {
  checkAbort = await runChecks(kernel, host, exports[memoryName], ${profile === "release"});
  if (decodes !== 0) throw new Error("Wasm operations decoded a string");
}
finally { TextDecoder.prototype.decode = decode; }
await checkAbort();
const secondAbort = await runChecks(secondKernel, secondHost, other[memoryName], ${profile === "release"});
await secondAbort(true);
await initStale({module_or_path:bytes, allocator_module:compiledAllocator});
const stale = staleKernel.dispatch(2);
staleKernel.cancel(stale);
assert.equal(stale.__wbg_ptr, 0);
assert.throws(() => staleKernel.resume(stale), ${profile === "release" ? "WebAssembly.RuntimeError" : "/null pointer/"},
  "a released task handle cannot be polled");
await initOom({module_or_path:bytes, allocator_module:readFileSync(${JSON.stringify(join(oom, "oom.wasm"))})});
assert.throws(() => oomKernel.allocation_check(47), WebAssembly.RuntimeError,
  "allocation failure traps without calling back into JS for diagnostics");
`;
  writeFileSync(join(pkg, "checks.mjs"), runner);
  execFileSync(process.env.ESBUILD ?? "esbuild", [join(pkg, "checks.mjs"), "--bundle", "--platform=node", "--format=esm", `--outfile=${join(pkg, "run.mjs")}`], {stdio: "inherit"});
  execFileSync(process.execPath, [join(pkg, "run.mjs")], {stdio: "inherit"});
  console.log(`${profile}: async bridge checks passed`);
}
