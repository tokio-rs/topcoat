export function install(bundles) {
  const expressions = new Map();
  for (const bundle of bundles) {
    for (const entry of bundle.entries) {
      if (!expressions.has(entry.id)) expressions.set(entry.id, { ...bundle, entry });
    }
  }
  globalThis.__topcoatWasm = (cx, id, captures) => {
    const expression = expressions.get(id);
    if (!expression) throw new Error(`Missing Wasm expression ${id}`);
    const { entry, dispatch, frames } = expression;
    if (entry.captures.length !== captures.length) throw new Error("Wasm capture arity mismatch");
    const run = () => {
      frames.push({ cx, captures, entry });
      try {
        const result = dispatch(entry.index);
        if (entry.result === "()") return undefined;
        const integer = entry.result.match(/^[iu](8|16|32)$/);
        return cx.hydrate(integer ? { t: entry.result, bits: Number(integer[1]), v: String(result) } : result);
      } finally {
        frames.pop();
      }
    };
    return entry.handler ? run : run();
  };
}
