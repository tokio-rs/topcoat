export function install(bundles) {
  const expressions = new Map();
  for (const bundle of bundles) {
    const state = { failure: null };
    for (const entry of bundle.entries) {
      if (!expressions.has(entry.id)) expressions.set(entry.id, { ...bundle, entry, state });
    }
  }
  globalThis.__topcoatWasm = (cx, id, captures) => {
    const expression = expressions.get(id);
    if (!expression) throw new Error(`Missing Wasm expression ${id}`);
    const { entry, dispatch, frames, state, procedures = [] } = expression;
    if (entry.captures.length !== captures.length) throw new Error("Wasm capture arity mismatch");
    const run = (event) => {
      if (state.failure) {
        if (entry.async) return Promise.reject(state.failure.error);
        throw state.failure.error;
      }
      const depth = frames.length;
      const current = { cx, captures, entry, event, procedures };
      frames.push(current);
      try {
        if (entry.async) return startTask(expression, current);
        const result = dispatch(entry.index);
        if (entry.result === "()") return undefined;
        const integer = entry.result.match(/^[iu](8|16|32)$/);
        return cx.hydrate(integer ? { t: entry.result, bits: Number(integer[1]), v: String(result) } : result);
      } catch (error) {
        if (error instanceof WebAssembly.RuntimeError) state.failure = { error };
        throw error;
      } finally {
        frames.length = depth;
      }
    };
    return entry.handler ? run : run();
  };
}

function startTask({ dispatch, resume, cancel, frames, state }, current) {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  const task = current.task = { id: null, wait: null, response: null, error: null, active: true };
  function finish(ok, error) {
    if (!task.active) return;
    task.active = false;
    task.wait?.catch(() => {});
    task.wait = task.response = null;
    try {
      if (task.id !== null) cancel(task.id);
    } catch (cleanupError) {
      // An abort does not unwind Wasm frames. Still settle the JS promise if
      // the generated binding cannot release a handle from the aborted call.
      if (ok) { ok = false; error = cleanupError; }
    }
    task.id = null;
    if (ok) resolve();
    else reject(error);
  }
  function drive() {
    if (state.failure) { finish(false, state.failure.error); return; }
    const depth = frames.length;
    frames.push(current);
    let ready;
    try {
      ready = resume(task.id);
    } catch (error) {
      // A thrown import or Wasm trap skips Rust destructors. Disable this
      // module; normal procedure rejection never passes through this path.
      state.failure = { error };
      finish(false, error);
      return;
    } finally {
      frames.length = depth;
    }
    if (state.failure) finish(false, state.failure.error);
    else if (task.error) finish(false, task.error);
    else if (ready) finish(true);
    else if (!task.wait) finish(false, new Error("Wasm handler suspended without a Topcoat procedure"));
    else {
      const operation = task.wait;
      operation.then(value => {
        // A completed/cancelled task or superseded operation cannot resume a
        // different Rust task. Promise callbacks always run after polling.
        if (!task.active || task.wait !== operation) return;
        task.wait = null;
        task.response = { value };
        drive();
      }, error => {
        if (task.active && task.wait === operation) finish(false, error ?? new Error("Procedure rejected"));
      });
    }
  }
  try {
    task.id = dispatch(current.entry.index);
    // Poll before returning to the listener, preserving preventDefault().
    drive();
  } catch (error) {
    state.failure = { error };
    finish(false, error);
  }
  return promise;
}
