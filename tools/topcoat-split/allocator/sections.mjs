import assert from "node:assert/strict";

// Preserve names for the allocator split while matching the existing release
// build's stripped target-feature metadata. wasm-bindgen uses that metadata to
// select its JS-reference representation, which otherwise changes this experiment.
export function withoutTargetFeatures(bytes) {
  let offset = 8;
  const chunks = [bytes.subarray(0, offset)];
  function uleb() {
    let value = 0, shift = 0, byte;
    do {
      assert.ok(offset < bytes.length && shift <= 28, "invalid Wasm section length");
      byte = bytes[offset++];
      value += (byte & 127) * 2 ** shift;
      shift += 7;
    } while (byte & 128);
    return value;
  }
  while (offset < bytes.length) {
    const start = offset, id = bytes[offset++], length = uleb(), end = offset + length;
    assert.ok(end <= bytes.length);
    let omit = false;
    if (id === 0) {
      const length = uleb();
      assert.ok(offset + length <= end);
      omit = bytes.subarray(offset, offset + length).toString("utf8") === "target_features";
    }
    if (!omit) chunks.push(bytes.subarray(start, end));
    offset = end;
  }
  return Buffer.concat(chunks);
}
