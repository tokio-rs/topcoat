import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";

const escapeRegExp = value => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

// Rewrite only the generated import keys and Wasm export accesses. Public JS
// names (such as dispatch) and the host module's API remain readable and stable.
export function renameGlue(glue, before, after) {
  const imports = WebAssembly.Module.imports(before);
  const renamedImports = WebAssembly.Module.imports(after);
  const exports = WebAssembly.Module.exports(before);
  const renamedExports = WebAssembly.Module.exports(after);
  assert.equal(imports.length, renamedImports.length);
  assert.equal(exports.length, renamedExports.length);
  const modules = new Map();
  const keys = new Map();
  imports.forEach((entry, index) => {
    const renamed = renamedImports[index];
    assert.equal(entry.kind, renamed.kind);
    if (modules.has(entry.module)) assert.equal(modules.get(entry.module), renamed.module);
    modules.set(entry.module, renamed.module);
    // wasm-bindgen's web target emits one import object. Fail closed if a
    // future version uses conflicting field names across multiple modules.
    if (keys.has(entry.name)) assert.equal(keys.get(entry.name), renamed.name);
    keys.set(entry.name, renamed.name);
  });
  for (const [name, renamed] of keys) {
    const key = new RegExp(`^(\\s*)${escapeRegExp(name)}:`, "gm");
    assert.equal([...glue.matchAll(key)].length, 1, `missing or ambiguous import key: ${name}`);
    glue = glue.replace(key, (_, indent) => `${indent}${renamed}:`);
  }
  for (const [name, renamed] of modules) {
    const key = `${JSON.stringify(name)}:`;
    assert.equal(glue.split(key).length, 2, `missing or ambiguous import module: ${name}`);
    glue = glue.replace(key, `${JSON.stringify(renamed)}:`);
  }
  // Replace in one pass: short new names must not be renamed a second time.
  const names = new Map(exports.map((entry, index) => {
    assert.equal(entry.kind, renamedExports[index].kind);
    return [entry.name, renamedExports[index].name];
  }));
  return glue.replace(/\bwasm\.([A-Za-z_$][\w$]*)\b/g, (access, name) => {
    assert.ok(names.has(name), `unknown Wasm export in glue: ${name}`);
    return `wasm.${names.get(name)}`;
  });
}

export function buildClient({ directory, targetDir, pkg, name, profile, client, wasmOpt }) {
  assert.ok(profile === "debug" || profile === "release");
  const release = profile === "release";
  const run = (command, args) => execFileSync(command, args, { stdio: "inherit" });
  run("cargo", [`+${client}`, "build", "--manifest-path", join(directory, "Cargo.toml"),
    ...(release ? ["--release"] : []), "--target", "wasm32-unknown-unknown", "--target-dir", targetDir]);
  // This directory contains only generated bindings. Avoid stale declarations
  // and host snippets when rebuilding with different options.
  rmSync(pkg, { recursive: true, force: true });
  run("wasm-bindgen", [join(targetDir, "wasm32-unknown-unknown", profile, `${name}.wasm`),
    "--target", "web", "--out-dir", pkg, "--out-name", "kernel", "--no-typescript",
    ...(release ? ["--remove-producers-section"] : ["--keep-debug"])]);
  const wasm = join(pkg, "kernel_bg.wasm");
  const js = join(pkg, "kernel.js");
  if (release) {
    const features = ["--enable-bulk-memory", "--enable-nontrapping-float-to-int"];
    run(wasmOpt, [wasm, "-O3", "-Oz", "--strip-debug", "--strip-producers", ...features, "-o", wasm]);
    const before = new WebAssembly.Module(readFileSync(wasm));
    assert.ok(!WebAssembly.Module.exports(before).some(entry => /malloc|realloc/.test(entry.name)),
      "typed release modules must not export byte allocation");
    const renamed = join(pkg, "renamed.wasm");
    execFileSync(wasmOpt, [wasm, ...features, "--minify-imports-and-exports-and-modules", "-o", renamed],
      { stdio: ["ignore", "pipe", "inherit"] });
    const bytes = readFileSync(renamed);
    const glue = renameGlue(readFileSync(js, "utf8"), before, new WebAssembly.Module(bytes));
    writeFileSync(js, glue);
    writeFileSync(wasm, bytes);
    rmSync(renamed);
  }
  return { wasm, glue: readFileSync(js, "utf8") };
}
