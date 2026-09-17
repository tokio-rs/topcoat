import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { withoutTargetFeatures } from "./allocator/sections.mjs";
import { allocatorTools, splitAllocator, importAllocatorMemory, buildAllocator, allocatorGlue } from "./allocator/build.mjs";

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
  const direct = new Map();
  for (const [name, renamed] of keys) {
    const key = new RegExp(`^(\\s*)${escapeRegExp(name)}:`, "gm");
    const matches = [...glue.matchAll(key)];
    if (matches.length === 1) {
      glue = glue.replace(key, (_, indent) => `${indent}${renamed}:`);
    } else {
      assert.equal(matches.length, 0, `ambiguous import key: ${name}`);
      const entry = imports.find(entry => entry.name === name);
      const fields = direct.get(entry.module) ?? [];
      fields.push([name, renamed]);
      direct.set(entry.module, fields);
    }
  }
  const merged = new Map();
  const lines = new Map();
  for (const [name, renamed] of modules) {
    const key = `${JSON.stringify(name)}:`;
    assert.equal(glue.split(key).length, 2, `missing or ambiguous import module: ${name}`);
    const pattern = new RegExp(`^[ \t]*${escapeRegExp(key)}\\s*(\\w+),$`, "m");
    const match = glue.match(pattern);
    assert.ok(match, `missing import namespace: ${name}`);
    const namespace = match[1];
    const value = direct.has(name)
      ? `{${direct.get(name).map(([old, key]) => `${JSON.stringify(key)}:${namespace}[${JSON.stringify(old)}]`).join(",")}}`
      : namespace;
    const values = merged.get(renamed) ?? [];
    values.push(value);
    merged.set(renamed, values);
    lines.set(match[0], renamed);
  }
  const emitted = new Set();
  glue = glue.split("\n").map(line => {
    const name = lines.get(line);
    if (!name) return line;
    if (emitted.has(name)) return "";
    emitted.add(name);
    return `        ${JSON.stringify(name)}: Object.assign({}, ${merged.get(name).join(", ")}),`;
  }).join("\n");
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

// Multiple Rust extern blocks can produce repeated namespace imports from
// the same host module. Keep one key in the instantiation import object.
function dedupeImportModules(glue) {
  const namespaces = new Map([...glue.matchAll(/import \* as (\w+) from ["']([^"']+)["']/g)].map(match => [match[1], match[2]]));
  const seen = new Map();
  return glue.replace(/^[ \t]*("[^"\n]+"):\s*(\w+),$/gm, (line, key, name) => {
    if (!seen.has(key)) { seen.set(key, name); return line; }
    assert.ok(namespaces.has(name));
    assert.equal(namespaces.get(name), namespaces.get(seen.get(key)));
    return "";
  });
}

export function buildClient({ directory, targetDir, pkg, name, profile, client, wasmOpt }) {
  assert.ok(profile === "debug" || profile === "release");
  const release = profile === "release";
  const run = (command, args) => execFileSync(command, args, { stdio: "inherit" });
  const { binaryen } = allocatorTools();
  run("cargo", [`+${client}`, "rustc", "--manifest-path", join(directory, "Cargo.toml"),
    ...(release ? ["--release"] : []), "--target", "wasm32-unknown-unknown", "--target-dir", targetDir,
    "--", "-Cstrip=none", "-Clink-arg=--no-stack-first", "-Clink-arg=--global-base=65536", "-Clink-arg=-zstack-size=1048576"]);
  // This directory contains only generated bindings. Avoid stale declarations
  // and host snippets when rebuilding with different options.
  rmSync(pkg, { recursive: true, force: true });
  const input = join(targetDir, "wasm32-unknown-unknown", profile, `${name}.wasm`);
  const bindingInput = `${input}.bindings.wasm`;
  writeFileSync(bindingInput, release ? withoutTargetFeatures(readFileSync(input)) : readFileSync(input));
  run("wasm-bindgen", [bindingInput,
    "--target", "web", "--out-dir", pkg, "--out-name", "kernel", "--no-typescript",
    "--keep-debug"]);
  const wasm = join(pkg, "kernel_bg.wasm");
  const js = join(pkg, "kernel.js");
  const memory = splitAllocator(wasm, join(pkg, "allocator.json"), binaryen);
  const features = ["--enable-bulk-memory", "--enable-nontrapping-float-to-int"];
  if (release) {
    run(process.execPath, [fileURLToPath(new URL("./abort.mjs", import.meta.url)), binaryen, wasm, join(pkg, "abort.json")]);
    run(wasmOpt, [wasm, "-O3", "-Oz", "--strip-debug", "--strip-producers", ...features, "-o", wasm]);
    run(process.execPath, [fileURLToPath(new URL("./trim.mjs", import.meta.url)), binaryen, wasm, js]);
  }
  // Optimization can remove every allocation, including binding support that
  // wasm-bindgen initially retained. Such bundles need no allocator download.
  const needsAllocator = WebAssembly.Module.imports(new WebAssembly.Module(readFileSync(wasm)))
    .some(entry => entry.module === "topcoat_allocator");
  const allocator = needsAllocator ? buildAllocator({ targetDir, client, profile, wasmOpt, binaryen }) : null;
  let glue = dedupeImportModules(readFileSync(js, "utf8"));
  if (WebAssembly.Module.exports(new WebAssembly.Module(readFileSync(wasm)))
    .some(entry => entry.name === "__wbg_task_free")) {
    // Explicit completion/cancellation and the binding's finalizer use one
    // Rust destruction path. Keep the bridge's private JS API unchanged.
    assert.ok(glue.includes("export class Task"));
    assert.ok(glue.includes("function _assertClass("));
    glue += "\nexport function cancel(task) { _assertClass(task, Task); task.free(); }\n";
  }
  if (allocator) {
    assert.ok(memory);
    importAllocatorMemory(wasm, binaryen, profile);
    glue = allocatorGlue(glue, pkg, allocator, memory);
  } else {
    writeFileSync(join(pkg, "allocator.json"), "null");
  }
  writeFileSync(js, glue);
  if (release) {
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
  return { wasm, glue: readFileSync(js, "utf8"), allocator };
}
