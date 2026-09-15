---
name: check
description: Always use this skill to verify a change locally before committing or opening a pull request in the Topcoat repository
---

# Verifying a Change

Keep this file in sync with CI workflows.

Run these by default:

```
cargo +nightly fmt --all # nightly is required, CI checks formatting with it
cargo topcoat fmt # formats Topcoat macros inside source files (ignore Leptos errors)
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="--cfg docsrs -Dwarnings" cargo +nightly doc --workspace --all-features --no-deps --locked
```

Only on user request:

```
# per-feature lint, catches feature combos that do not build (needs cargo-hack)
cargo hack clippy --workspace --each-feature --exclude-features stage-icons --no-dev-deps -- -D warnings

# unused dependencies, which CI fails on (needs cargo-udeps on nightly)
cargo +nightly udeps --workspace --all-targets --all-features --locked
```

## Runtime browser bundle

Only when you touched `crates/topcoat-runtime/browser`. The runtime serves a prebuilt `dist/index.js` via `asset!`, and the coherence crate embeds `dist/coherence.js`. CI rejects drift in either bundle (`git diff --exit-code -- dist/index.js dist/coherence.js`). Rebuild them and stage any regenerated bundles alongside your source change:

```
cd crates/topcoat-runtime/browser
yarn install --frozen-lockfile
yarn build
yarn test
```

## New crates

A new crate must be referenced in the toplevel `Cargo.toml` as well as `release-plz.toml`.
