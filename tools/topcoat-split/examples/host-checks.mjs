import assert from "node:assert/strict";
import { frames, capture, read, write, text_concat, text_empty } from "../src/wasm/host.js";

export function checkHost() {
  const values = ["\u2615\u{1f680}e\u0301\0<&>", true, null, 3.5, -128, -32768, -2147483648, 255, 65535, 4294967295];
  const kinds = ["Text", "bool", "()", "f64", "i8", "i16", "i32", "u8", "u16", "u32"];
  const cx = { hydrate: value => ({ dehydrate: () => value }) };
  const captures = values.map(value => {
    let current = cx.hydrate({ t: "Wasm", v: value });
    return {
      dehydrate: () => current.dehydrate(),
      get: () => current,
      set: value => { current = value; },
    };
  });
  frames.push({ cx, captures, entry: { captures: kinds.map(bridge => ({ bridge })) } });
  try {
    values.forEach((value, slot) => {
      assert.equal(capture(slot), value);
      assert.equal(read(slot), value);
      write(slot, value);
      assert.equal(read(slot), value);
    });
    for (const [slot, invalid] of [[0, 42], [0, "\ud800"], [0, "\udc00"], [1, 1], [2, undefined], [3, NaN], [3, Infinity], [4, -129], [4, 128], [5, -32769], [6, 2147483648], [7, -1], [7, 256], [8, 65536], [9, 4294967296], [9, 1.5]]) {
      assert.throws(() => write(slot, invalid), TypeError);
      assert.equal(read(slot), values[slot], "invalid writes leave the signal unchanged");
    }
    assert.equal(text_concat(values[0], values[0]), values[0] + values[0]);
    assert.equal(text_empty(""), true);
    assert.equal(text_empty("\0"), false);
  } finally {
    frames.pop();
  }
  assert.throws(() => read(0), /outside a capture frame/);
  console.log("Typed host checks passed: primitive bounds, Unicode, and invalid writes.");
}
