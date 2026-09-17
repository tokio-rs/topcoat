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
  const kind = frame().entry.captures[slot]?.bridge;
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
  if (!valid) throw new TypeError(`Invalid ${kind} value at Wasm capture ${slot}`);
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
