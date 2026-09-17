export const frames = [];
function frame() {
  const current = frames.at(-1);
  if (!current) throw new Error("Wasm expression called outside a capture frame");
  return current;
}

// Text contains Unicode scalar values on both the server and the client.
// Check UTF-16 without encoding or copying it into Wasm memory.
function wellFormed(value) {
  for (let i = 0; i < value.length; i++) {
    const code = value.charCodeAt(i);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(++i);
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      return false;
    }
  }
  return true;
}

function checked(slot, value) {
  return checkedValue(frame().entry.captures[slot]?.bridge, value);
}

function checkedValue(kind, value) {
  let valid = false;
  if (kind === "Text") valid = typeof value === "string" && wellFormed(value);
  else if (kind === "bool") valid = typeof value === "boolean";
  else if (kind === "()") valid = value === null;
  else if (kind === "f64") valid = typeof value === "number" && Number.isFinite(value);
  else if (/^[iu](8|16|32)$/.test(kind)) {
    const bits = Number(kind.slice(1));
    const signed = kind[0] === "i";
    const min = signed ? -(2 ** (bits - 1)) : 0;
    const max = 2 ** (bits - (signed ? 1 : 0)) - 1;
    valid = Number.isInteger(value) && value >= min && value <= max;
  }
  if (!valid) throw new TypeError(`Invalid ${kind} value at Wasm boundary`);
  return value;
}

export function capture(slot) {
  return checked(slot, frame().captures[slot].dehydrate().v);
}
export function read(slot) {
  return checked(slot, frame().captures[slot].get().dehydrate().v);
}
export function write(slot, value) {
  const current = frame();
  checked(slot, value);
  current.captures[slot].set(current.cx.hydrate({ t: "Wasm", v: value }));
}
export function text_concat(left, right) { return left + right; }
export function text_empty(value) { return value.length === 0; }

// Keep validation before wasm-bindgen coerces numbers and booleans at the ABI.
export function capture_number(slot) { return capture(slot); }
export function read_number(slot) { return read(slot); }
export function write_number(slot, value) { write(slot, value); }
export function capture_bool(slot) { return capture(slot); }
export function read_bool(slot) { return read(slot); }
export function write_bool(slot, value) { write(slot, value); }

const eventFields = ["alt_key", "bubbles", "button", "buttons", "cancelable", "client_x", "client_y", "code", "ctrl_key", "data", "default_prevented", "delta_x", "delta_y", "delta_z", "event_type", "input_type", "is_composing", "key", "meta_key", "movement_x", "movement_y", "offset_x", "offset_y", "page_x", "page_y", "pointer_id", "pointer_type", "repeat", "screen_x", "screen_y", "shift_key", "time_stamp", "target.checked", "target.id", "target.name", "target.text_content", "target.value", "current_target.checked", "current_target.id", "current_target.name", "current_target.text_content", "current_target.value"];
export function snapshotEvent(event) {
  return eventFields.map(path => path.split(".").reduce((value, key) => value[key], event).dehydrate());
}
export function event_text(field) {
  const value = (frame().eventValues ??= snapshotEvent(frame().event))[field];
  if (typeof value !== "string" || !wellFormed(value)) throw new TypeError("Invalid event text");
  return value;
}
export function event_number(field) { return (frame().eventValues ??= snapshotEvent(frame().event))[field]; }
export function event_bool(field) { return (frame().eventValues ??= snapshotEvent(frame().event))[field]; }
export function event_action(action) {
  frame().event[["prevent_default", "stop_propagation", "stop_immediate_propagation"][action]]();
}

// A task can have one outstanding Topcoat procedure. Scheduling and results
// stay in JS; Rust only polls its compiler-generated future when resumed.
export function arguments_new() { return []; }
export function arguments_push(args, value) { args.push(value); }
export function procedure_start(index, args) {
  const task = frame().task;
  if (task.wait || task.response) {
    task.error = new Error("Concurrent procedure waits are not supported by the Wasm bridge");
    return;
  }
  task.wait = procedure_call(index, args);
}
export function procedure_ready() { return frame().task.response !== null; }
export function procedure_result() {
  const task = frame().task;
  const value = task.response.value;
  task.response = null;
  return value;
}
function wire(kind, value) {
  checkedValue(kind, value);
  return /^[iu](8|16|32)$/.test(kind) ? {t: kind, bits: Number(kind.slice(1)), v: String(value)} : value;
}
export async function procedure_call(index, args) {
  const current = frame();
  const procedure = current.procedures.find(procedure => procedure.index === index);
  if (!procedure || args.length !== procedure.args.length) throw new Error("Invalid Wasm procedure call");
  // The existing runtime owns HTTP, JSON, error handling, and hydration.
  const callable = current.cx.hydrate({t: "Procedure", id: procedure.id});
  const result = await callable.call(...args.map((value, index) => current.cx.hydrate(wire(procedure.args[index], value))));
  const dehydrated = result?.dehydrate() ?? null;
  const value = /^[iu](8|16|32)$/.test(procedure.result) ? Number(dehydrated.v) : dehydrated;
  return checkedValue(procedure.result, value);
}
