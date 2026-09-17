import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync, copyFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const directory = dirname(fileURLToPath(import.meta.url));
const built = new Map();
const run = (command, args) => execFileSync(command, args, { stdio: "inherit" });

export function allocatorTools() {
  return { binaryen: process.env.BINARYEN_JS ?? createRequire(import.meta.url).resolve("binaryen") };
}

export function splitAllocator(wasm, metadata, binaryen) {
  run(process.execPath, [join(directory, "split.mjs"), binaryen, "client", wasm, metadata]);
  return JSON.parse(readFileSync(metadata, "utf8"));
}

export function importAllocatorMemory(wasm, binaryen, profile) {
  run(process.execPath, [join(directory, "split.mjs"), binaryen, "memory", wasm, profile]);
}

export function buildAllocator({ targetDir, client, profile, wasmOpt, binaryen }) {
  const key = JSON.stringify([targetDir, client, profile, wasmOpt, binaryen]);
  if (built.has(key)) return built.get(key);
  const root = join(targetDir, "shared-allocator");
  mkdirSync(join(root, "src"), { recursive: true });
  const cargo = readFileSync(join(directory, "../src/wasm.rs"), "utf8").match(/pub const CARGO: &str = r#"([\s\S]*?)"#;/)[1];
  writeFileSync(join(root, "Cargo.toml"), `[workspace]\n[package]\nname="topcoat-shared-allocator"\nversion="0.0.0"\nedition="2024"\n${cargo}`);
  // Safe Rust retains the dependency's allocation implementation. Fallible
  // reservation keeps diagnostic formatting out of the seed's function table.
  // The linker exposes the allocator methods, not this retention function.
  writeFileSync(join(root, "src/lib.rs"), `#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;
${readFileSync(join(directory, "../src/wasm/allocator.rs.txt"), "utf8")}
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub fn allocator_address() -> usize { &ALLOCATOR as *const _ as usize }
#[wasm_bindgen]
pub fn retain_allocator(size: usize) {
    let mut bytes = alloc::vec::Vec::<u8>::new();
    let _ = bytes.try_reserve_exact(size);
    core::hint::black_box(bytes);
}
`);
  run("cargo", [`+${client}`, "rustc", "--manifest-path", join(root, "Cargo.toml"),
    ...(profile === "release" ? ["--release"] : []), "--target", "wasm32-unknown-unknown", "--target-dir", join(root, "target"),
    "--", "-Cstrip=none", "-Clink-arg=--no-stack-first", "-Clink-arg=--global-base=1024", "-Clink-arg=-zstack-size=16384"]);
  const pkg = join(root, profile);
  run("wasm-bindgen", [join(root, "target/wasm32-unknown-unknown", profile, "topcoat_shared_allocator.wasm"),
    "--target", "web", "--out-dir", pkg, "--out-name", "allocator", "--no-typescript", "--keep-debug"]);
  const file = join(pkg, "allocator_bg.wasm");
  run(process.execPath, [join(directory, "split.mjs"), binaryen, "allocator", file]);
  if (profile === "release") run(wasmOpt, [file, "-O3", "-Oz", "--strip-debug", "--strip-producers", "--strip-target-features",
    "--enable-bulk-memory", "--enable-nontrapping-float-to-int", "-o", file]);
  const bytes = readFileSync(file);
  const module = new WebAssembly.Module(bytes);
  assert.deepEqual(WebAssembly.Module.imports(module), [{ module: "env", name: "memory", kind: "memory" }]);
  assert.deepEqual(WebAssembly.Module.exports(module).map(item => item.name).sort(), ["a", "d"]);
  const filename = `allocator-${createHash("sha256").update(bytes).digest("hex").slice(0, 16)}.wasm`;
  const result = { file, filename };
  built.set(key, result);
  return result;
}

export function allocatorGlue(glue, pkg, allocator, memory) {
  // All bundles under the same pkg directory import the same loader module.
  copyFileSync(join(directory, "loader.js"), join(pkg, "../allocator-loader.js"));
  copyFileSync(allocator.file, join(pkg, allocator.filename));
  writeFileSync(join(pkg, "allocator.js"), `import {compileAllocator, instantiateAllocator} from "../allocator-loader.js";
let memory, a, d;
export { memory as __topcoat_memory, a as __topcoat_allocate, d as __topcoat_deallocate };
export function initializeSync(module) {
    if (memory) return;
    if (!(module instanceof WebAssembly.Module)) module = new WebAssembly.Module(module);
    ({memory, a, d} = instantiateAllocator(module, ${memory.initial}));
}
export async function initialize(source) {
    initializeSync(await compileAllocator(source));
}
`);
  const marker = "function __wbg_get_imports() {";
  assert.equal(glue.split(marker).length, 2);
  // This namespace follows the same shape as wasm-bindgen's direct imports,
  // so the existing release import-name minifier also handles it.
  const start = glue.indexOf(marker), end = glue.indexOf("\n}", start);
  const before = glue.slice(start, end);
  const replaced = before.replace(/return \{/, 'return {\n        "topcoat_allocator": allocator,');
  assert.notEqual(before, replaced);
  glue = glue.slice(0, start) + replaced + glue.slice(end);
  glue = glue.replace("function initSync(module) {", "function initSync(module) {\n    const allocatorModule = module?.allocator_module;");
  glue = glue.replace("async function __wbg_init(module_or_path) {", "async function __wbg_init(module_or_path) {\n    const allocatorModule = module_or_path?.allocator_module;");
  const imports = "    const imports = __wbg_get_imports();";
  assert.equal(glue.split(imports).length, 3);
  let count = 0;
  glue = glue.replaceAll(imports, () => (++count === 1
    ? "    allocator.initializeSync(allocatorModule);\n"
    : `    await allocator.initialize(allocatorModule ?? new URL('./${allocator.filename}', import.meta.url));\n`) + imports);
  return 'import * as allocator from "./allocator.js";\n' + glue;
}
