import { buildClient } from "../build-client.mjs";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync, brotliCompressSync } from "node:zlib";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const args = process.argv.slice(2);
if (args.length > 1 || args.some(arg => !["--debug", "--release"].includes(arg))) {
  throw new Error("Usage: node tools/topcoat-split/examples/coffee-shop.mjs [--debug|--release]");
}
const profile = args[0] === "--debug" ? "debug" : "release";
const output = join(root, profile === "release" ? "target/coffee-shop-wasm" : "target/coffee-shop-wasm-debug");
const cache = join(root, "target/topcoat-native");
const tool = join(root, "tools/topcoat-split");
const analyzer = process.env.TOPCOAT_SPLIT_ANALYZER_TOOLCHAIN ?? "nightly-2026-08-05";
const client = process.env.TOPCOAT_SPLIT_CLIENT_TOOLCHAIN ?? "stable";
const run = (command, args, options = {}) => execFileSync(command, args, { cwd: root, stdio: "inherit", ...options });
mkdirSync(join(output, "public"), { recursive: true });
run("cargo", [`+${analyzer}`, "build", "--manifest-path", join(tool, "Cargo.toml")]);
const native = { ...process.env, RUSTFLAGS: `${process.env.RUSTFLAGS ?? ""} --cfg topcoat_wasm` };
run("cargo", [`+${analyzer}`, "rustc", "-p", "coffee-shop", "--bin", "coffee-shop", "--", "--cfg", `topcoat_wasm_run="${Date.now()}"`], {
  env: { ...native, CARGO_TARGET_DIR: join(cache, "analysis"), RUSTC_WORKSPACE_WRAPPER: join(tool, "target/debug/topcoat-split"),
    TOPCOAT_WASM_OUT: join(output, "crates"), TOPCOAT_WASM_CRATE: "coffee_shop", TOPCOAT_WASM_ANALYSIS: "1" },
});
const manifestPath = join(output, "crates/manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const imports = [], browserInit = [], nodeInit = [], sizes = [];
const allocatorModules = new Map();
for (const [index, bundle] of manifest.bundles.entries()) {
  const pkg = join(output, "pkg", bundle.name);
  const clientCargo = readFileSync(join(output, "crates", bundle.name, "Cargo.toml"), "utf8");
  if (/wasm-bindgen-futures|js-sys|serde/.test(clientCargo)) throw new Error("client crate must use only the typed host bridge");
  const {wasm, glue, allocator} = buildClient({ directory: join(output, "crates", bundle.name), targetDir: join(cache, "client"), pkg,
    name: `topcoat_client_${bundle.name.replaceAll("-", "_")}`, profile, client, wasmOpt: process.env.WASM_OPT ?? "wasm-opt" });
  if (allocator && !allocatorModules.has(allocator.filename)) {
    const name = `allocator${allocatorModules.size}`;
    allocatorModules.set(allocator.filename, name);
    nodeInit.push(`const ${name} = new WebAssembly.Module(readFileSync(${JSON.stringify(allocator.file)}));`);
  }
  const host = glue.match(/from ['"]([^'"]+\/host\.js)['"]/);
  if (!host) throw new Error("missing host imports");
  imports.push(`import init${index}, {dispatch as dispatch${index}${bundle.entries.some(entry => entry.async) ? `, resume as resume${index}, cancel as cancel${index}` : ""}} from ${JSON.stringify(join(pkg, "kernel.js"))};\nimport {frames as frames${index}} from ${JSON.stringify(join(pkg, host[1]))};`);
  browserInit.push(`await init${index}({module_or_path: new URL(${JSON.stringify(`./${bundle.name}.wasm`)}, import.meta.url)});`);
  nodeInit.push(`await init${index}({module_or_path: readFileSync(${JSON.stringify(wasm)})${allocator ? `, allocator_module: ${allocatorModules.get(allocator.filename)}` : ""}});`);
  if (allocator && !sizes.some(item => item.bundle === allocator.filename)) {
    const bytes = readFileSync(allocator.file);
    writeFileSync(join(output, "public", allocator.filename), bytes);
    sizes.push({ bundle: allocator.filename, shared: true, profile, raw: bytes.length, gzip: gzipSync(bytes).length, brotli: brotliCompressSync(bytes).length });
  }
  const bytes = readFileSync(wasm);
  writeFileSync(join(output, "public", `${bundle.name}.wasm`), bytes);
  sizes.push({bundle: bundle.name, expressions: bundle.entries.length, profile, raw: bytes.length, gzip: gzipSync(bytes).length, brotli: brotliCompressSync(bytes).length});
}
const bridge = `import {install} from ${JSON.stringify(join(tool, "src/wasm/bridge.js"))};\ninstall([${manifest.bundles.map((bundle,index) => `{entries:${JSON.stringify(bundle.entries)},procedures:${JSON.stringify(bundle.procedures)},dispatch:dispatch${index},frames:frames${index}${bundle.entries.some(entry => entry.async) ? `,resume:resume${index},cancel:cancel${index}` : ""}}`).join(",")}]);`;
writeFileSync(join(output, "app.ts"), `${imports.join("\n")}\n${browserInit.join("\n")}\n${bridge}\nimport {Runtime} from ${JSON.stringify(join(root, "crates/topcoat-runtime/browser/src/runtime.ts"))};\nnew Runtime().start(document);`);
const esbuild = process.env.ESBUILD ?? "esbuild";
run(esbuild, [join(output,"app.ts"), "--bundle", "--format=esm", ...(profile === "release" ? ["--minify"] : ["--sourcemap"]), `--outfile=${join(output,"public/app.js")}`]);
writeFileSync(join(output,"checks.ts"), `import {readFileSync} from "node:fs";\n${imports.join("\n")}\n${nodeInit.join("\n")}\n${bridge}\nimport {runChecks} from ${JSON.stringify(join(tool,"examples/coffee-shop-checks.ts"))};\nawait runChecks(${JSON.stringify(manifest)}, [${manifest.bundles.map((_,index) => `frames${index}`).join(",")}]);`);
run(esbuild, [join(output,"checks.ts"), "--bundle", "--platform=node", "--format=esm", `--external:${join(root,"crates/topcoat-runtime/browser/node_modules/*")}`, `--outfile=${join(output,"checks.mjs")}`]);
run("cargo", [`+${client}`, "rustc", "-p", "coffee-shop", "--bin", "coffee-shop", "--", "--cfg", `topcoat_wasm_run="${Date.now()}"`], {
  env: { ...native, CARGO_TARGET_DIR: join(cache,"server"), TOPCOAT_WASM_MANIFEST: manifestPath },
});
run(process.env.TOPCOAT_CLI ?? join(root,"target/debug/cargo-topcoat"), ["asset", "bundle", "--executable", join(cache,"server/debug/coffee-shop"), "--out", join(cache,"server/debug/assets")], {
  env: { ...native, CARGO_TARGET_DIR: join(cache,"server"), TOPCOAT_WASM_MANIFEST: manifestPath },
});
writeFileSync(join(output,"sizes.json"), JSON.stringify(sizes,null,2));
console.table(sizes);
console.log(`Run: TOPCOAT_WASM_PUBLIC=${join(output,"public")} PORT=8083 ${join(cache,"server/debug/coffee-shop")}`);
console.log(`Then test: COFFEE_URL=http://127.0.0.1:8083 node ${join(output,"checks.mjs")}`);
