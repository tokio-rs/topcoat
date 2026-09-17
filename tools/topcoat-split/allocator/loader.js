// Each compiled allocator module is reused, but each page gets its own instance
// and memory. Failed downloads can be retried.
const compiled = new Map();

export async function compileAllocator(source) {
  if (source instanceof WebAssembly.Module) return source;
  const key = source instanceof URL ? source.href : source;
  if (!compiled.has(key)) {
    const pending = (async () => {
      if (typeof key === "string") {
        const response = await fetch(key);
        if (!response.ok) throw new Error(`Allocator download failed: ${response.status}`);
        return WebAssembly.compile(await response.arrayBuffer());
      }
      return WebAssembly.compile(source);
    })();
    compiled.set(key, pending);
    pending.catch(() => { if (compiled.get(key) === pending) compiled.delete(key); });
  }
  return compiled.get(key);
}

export function instantiateAllocator(module, initial) {
  const memory = new WebAssembly.Memory({ initial });
  const instance = new WebAssembly.Instance(module, { env: { memory } });
  return { memory, a: instance.exports.a, d: instance.exports.d };
}
