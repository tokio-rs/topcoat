import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { brotliCompressSync, gzipSync } from "node:zlib";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const tool = join(root, "tools/topcoat-split");
const output = join(root, "target/driver-split");
const analyzer = process.env.TOPCOAT_SPLIT_ANALYZER_TOOLCHAIN ?? "nightly-2026-08-05";
const client = process.env.TOPCOAT_SPLIT_CLIENT_TOOLCHAIN ?? "stable";
const run = (command, args) => execFileSync(command, args, { cwd: root, stdio: "inherit" });

run("cargo", [`+${analyzer}`, "run", "--manifest-path", join(tool, "Cargo.toml"), "--",
  "--out-dir", output, "--", join(tool, "tests/fixtures/product.rs"), "--edition=2024"]);

const sizes = {};
for (const name of ["page", "shard"]) {
  const directory = join(output, `${name}-adapter`);
  mkdirSync(join(directory, "src"), { recursive: true });
  const manifest = JSON.parse(readFileSync(join(output, name, "manifest.json"), "utf8"));
  // This adapter is specific to the Product fixture. Extraction emits typed
  // Rust entry points; signal storage and the browser ABI are separate work.
  const arms = manifest.entries.map(entry =>
    `${entry.index} => client::${entry.function}(&signal) as f64,`).join("\n");
  writeFileSync(join(directory, "Cargo.toml"), `[workspace]
[package]
name = "${name}-adapter"
version = "0.0.0"
edition = "2024"
publish = false
[lib]
crate-type = ["cdylib"]
[dependencies]
client = { package = "topcoat-client-${name}", path = "../${name}" }
wasm-bindgen = "=0.2.128"
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
`);
  writeFileSync(join(directory, "src/lib.rs"), `#![forbid(unsafe_code)]
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub fn evaluate(index: u32, price: f64, quantity: u32, discount: f64) -> f64 {
    let signal = client::Signal { value: client::models::Product { price, quantity, discount } };
    match index { ${arms} _ => f64::NAN }
}
`);
  run("cargo", [`+${client}`, "build", "--release", "--target", "wasm32-unknown-unknown",
    "--manifest-path", join(directory, "Cargo.toml"), "--target-dir", join(output, "build")]);
  const pkg = join(output, "pkg", name);
  run("wasm-bindgen", [join(output, "build/wasm32-unknown-unknown/release", `${name}_adapter.wasm`),
    "--target", "nodejs", "--out-dir", pkg, "--out-name", "kernel"]);
  if (process.env.WASM_OPT) {
    run(process.env.WASM_OPT, [join(pkg, "kernel_bg.wasm"), "-O3", "-Oz", "--strip-debug",
      "--enable-bulk-memory", "--enable-nontrapping-float-to-int", "-o", join(pkg, "kernel_bg.wasm")]);
  }
  const kernel = createRequire(import.meta.url)(join(pkg, "kernel.js"));
  if (name === "page") {
    assert.equal(kernel.evaluate(0, 4.5, 2, 0.1), 4.5);
    assert.equal(kernel.evaluate(1, 4.5, 2, 0.1), 8.1);
    assert.equal(kernel.evaluate(1, 9, 3, 0.25), 20.25);
  } else {
    assert.equal(kernel.evaluate(0, 4.5, 7, 0.1), 7);
  }
  const wasm = readFileSync(join(pkg, "kernel_bg.wasm"));
  sizes[name] = { wasm: wasm.length, gzip: gzipSync(wasm).length, brotli: brotliCompressSync(wasm).length };
}
writeFileSync(join(output, "sizes.json"), JSON.stringify({ analyzer, client, optimized: Boolean(process.env.WASM_OPT), files: sizes }, null, 2));
console.log("Both extracted bundles executed successfully in Wasm.");
console.table(sizes);
