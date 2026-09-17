import assert from "node:assert/strict";
import { buildClient } from "../build-client.mjs";
import { checkHost } from "./host-checks.mjs";
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync, brotliCompressSync } from "node:zlib";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");
const args = process.argv.slice(2);
if (args.length > 1 || args.some(arg => !["--debug", "--release"].includes(arg))) {
  throw new Error("Usage: node tools/topcoat-split/examples/topcoat.mjs [--debug|--release]");
}
const profile = args[0] === "--debug" ? "debug" : "release";
const release = profile === "release";
const output = join(root, release ? "target/topcoat-native" : "target/topcoat-native-debug");
const buildCache = join(root, "target/topcoat-native");
const wasmOpt = process.env.WASM_OPT ?? "wasm-opt";
if (release) execFileSync(wasmOpt, ["--version"], { stdio: "inherit" });
const tool = join(root, "tools/topcoat-split");
const analyzer = process.env.TOPCOAT_SPLIT_ANALYZER_TOOLCHAIN ?? "nightly-2026-08-05";
const client = process.env.TOPCOAT_SPLIT_CLIENT_TOOLCHAIN ?? "stable";
const run = (command, args, options = {}) => execFileSync(command, args, { cwd: root, stdio: "inherit", ...options });
mkdirSync(output, { recursive: true });
checkHost();
run("cargo", [`+${analyzer}`, "build", "--manifest-path", join(tool, "Cargo.toml")]);
const nativeEnv = { ...process.env, RUSTFLAGS: `${process.env.RUSTFLAGS ?? ""} --cfg topcoat_wasm` };
run("cargo", [`+${analyzer}`, "rustc", "-p", "runtime", "--bin", "wasm-native", "--", "--cfg", `topcoat_wasm_run="${Date.now()}"`], {
  env: { ...nativeEnv, CARGO_TARGET_DIR: join(buildCache, "analysis"),
    RUSTC_WORKSPACE_WRAPPER: join(tool, "target/debug/topcoat-split"),
    TOPCOAT_WASM_OUT: join(output, "crates"), TOPCOAT_WASM_CRATE: "wasm_native", TOPCOAT_WASM_ANALYSIS: "1" },
});
const manifestPath = join(output, "crates/manifest.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const imports = [];
const initializers = [];
const nodeInitializers = [];
const sizes = [];
const allocatorModules = new Map();
for (const [index, bundle] of manifest.bundles.entries()) {
  const directory = join(output, "crates", bundle.name);
  const pkg = join(output, "pkg", bundle.name);
  const { wasm, glue, allocator } = buildClient({ directory, targetDir: join(buildCache, "client"), pkg,
    name: `topcoat_client_${bundle.name.replaceAll("-", "_")}`, profile, client, wasmOpt });
  if (release) {
    assert.doesNotMatch(glue, /TextEncoder|TextDecoder|JSON\.(parse|stringify)/, "typed Wasm glue must not encode text or JSON");
  }
  const generatedCargo = readFileSync(join(directory, "Cargo.toml"), "utf8");
  assert.doesNotMatch(generatedCargo, /serde/, "client crates must not depend on serde");
  if (allocator && !allocatorModules.has(allocator.filename)) {
    const name = `allocator${allocatorModules.size}`;
    allocatorModules.set(allocator.filename, name);
    nodeInitializers.push(`const ${name} = new WebAssembly.Module(readFileSync(${JSON.stringify(allocator.file)}));`);
  }
  const host = glue.match(/from ['"]([^'"]+\/host\.js)['"]/);
  if (!host) throw new Error("missing Wasm host imports");
  imports.push(`import init${index}, {dispatch as dispatch${index}${bundle.entries.some(entry => entry.async) ? `, resume as resume${index}, cancel as cancel${index}` : ""}} from ${JSON.stringify(join(pkg, "kernel.js"))};\nimport {frames as frames${index}} from ${JSON.stringify(join(pkg, host[1]))};`);
  initializers.push(`await init${index}({module_or_path: new URL(${JSON.stringify(`./${bundle.name}.wasm`)}, import.meta.url)});`);
  nodeInitializers.push(`await init${index}({module_or_path: readFileSync(${JSON.stringify(wasm)})${allocator ? `, allocator_module: ${allocatorModules.get(allocator.filename)}` : ""}});`);
  const publicDir = join(output, "public");
  mkdirSync(publicDir, { recursive: true });
  if (allocator && !sizes.some(item => item.bundle === allocator.filename)) {
    const bytes = readFileSync(allocator.file);
    writeFileSync(join(output, "public", allocator.filename), bytes);
    sizes.push({ bundle: allocator.filename, shared: true, profile, raw: bytes.length, gzip: gzipSync(bytes).length, brotli: brotliCompressSync(bytes).length });
  }
  const bytes = readFileSync(wasm);
  writeFileSync(join(publicDir, `${bundle.name}.wasm`), bytes);
  sizes.push({ bundle: bundle.name, profile, raw: bytes.length, gzip: gzipSync(bytes).length, brotli: brotliCompressSync(bytes).length });
}
run("cargo", [`+${client}`, "rustc", "-p", "runtime", "--bin", "wasm-native", "--", "--cfg", `topcoat_wasm_run="${Date.now()}"`], {
  env: { ...nativeEnv, CARGO_TARGET_DIR: join(buildCache, "server"), TOPCOAT_WASM_MANIFEST: manifestPath },
});
const html = run(join(buildCache, "server/debug/wasm-native"), [], { stdio: ["ignore", "pipe", "inherit"], encoding: "utf8" });
writeFileSync(join(output, "public/index.html"), html);
const bridge = `import {install} from ${JSON.stringify(join(tool, "src/wasm/bridge.js"))};\ninstall([${manifest.bundles.map((bundle, index) => `{entries:${JSON.stringify(bundle.entries)},procedures:${JSON.stringify(bundle.procedures)},dispatch:dispatch${index},frames:frames${index}${bundle.entries.some(entry => entry.async) ? `,resume:resume${index},cancel:cancel${index}` : ""}}`).join(",")}]);`;
const runtime = join(root, "crates/topcoat-runtime/browser/src");
const app = `${imports.join("\n")}\n${initializers.join("\n")}\n${bridge}\nimport {Runtime} from ${JSON.stringify(join(runtime, "runtime.ts"))};\nnew Runtime().start(document);`;
writeFileSync(join(output, "app.ts"), app);
const esbuild = process.env.ESBUILD ?? "esbuild";
run(esbuild, [join(output, "app.ts"), "--bundle", "--format=esm", ...(release ? ["--minify"] : ["--sourcemap"]), `--outfile=${join(output, "public/app.js")}`]);
const test = `import {readFileSync} from "node:fs";\n${imports.join("\n")}\n${nodeInitializers.join("\n")}\n${bridge}\nimport {runChecks} from ${JSON.stringify(join(tool, "examples/topcoat-checks.ts"))};\nawait runChecks(${JSON.stringify(html)}, ${JSON.stringify(manifest)}, [${manifest.bundles.map((_, index) => `frames${index}`).join(",")}]);`;
writeFileSync(join(output, "checks.ts"), test);
run(esbuild, [join(output, "checks.ts"), "--bundle", "--platform=node", "--format=esm", `--external:${join(root, "crates/topcoat-runtime/browser/node_modules/*")}`, `--outfile=${join(output, "checks.mjs")}`]);
run(process.execPath, [join(output, "checks.mjs")]);
writeFileSync(join(output, "sizes.json"), JSON.stringify(sizes, null, 2));
console.table(sizes);
console.log(`Serve ${join(output, "public")} to try real Topcoat expressions in Wasm.`);
