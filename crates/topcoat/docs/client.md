The experimental Wasm backend uses explicit client value types so browser expressions can exchange values with JavaScript without JSON or UTF-8 conversion. This backend currently uses the compiler driver and build script in `tools/topcoat-split`; ordinary Topcoat builds use the JavaScript expression backend.

## Text

[`Text`] is immutable Unicode text. On the server it owns a native string; in Wasm it holds a reference to a JavaScript string. Its supported client operations are cloning, equality, concatenation, and checking whether it is empty. These operations preserve contents, including combining characters, without normalization.

```rust
use topcoat::client::Text;

let name = Text::from("Coffee");
let suffix = Text::from("!");
let label = name.concat(&suffix);
assert_eq!(label, Text::from("Coffee!"));
assert_eq!(name, Text::from("Coffee"));
assert!(!label.is_empty());
```

Construct Text on the server, then capture it in an expression or store it in a signal. Constructing Text from a string literal inside a Wasm expression is not supported yet. Text does not expose `&str`, byte indexing, or implicit conversion to a native String. Both backends use Unicode scalar values; malformed JavaScript strings are rejected at the bridge.

## Signal values

[`ClientValue`] identifies supported values for the experimental typed bridge. Its implementations are Text, bool, unit, f64, and fixed-width integers up to 32 bits. The trait is sealed while the bridge vocabulary is being established. Custom structs, collections, and native String values are not supported at this boundary, and there is no automatic JSON fallback.

Signals keep their identity in JavaScript. Reading a signal tracks a reactive dependency and returns a primitive or Text snapshot. A Text snapshot is immutable, so concatenating it does not change the signal; assigning the result with `set` performs the update. Integer bounds and value types are checked before a write changes the signal. Floating-point signal state must be finite.

Server-to-browser state transfer still uses serialization. Only the browser-to-Wasm execution path avoids it. The generated client crates do not depend on serde.
