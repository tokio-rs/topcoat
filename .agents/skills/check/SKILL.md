---
name: check
description: Always use this skill to verify a change locally before committing or opening a pull request in the Topcoat repository
---

# Verifying a Change

Keep this file in sync with CI workflows.

Run these by default:

```
cargo +nightly fmt --all # stable fmt is fine if nightly is missing
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

Only when you touched `crates/topcoat-runtime/browser`. The crate serves a prebuilt `dist/index.js` via `asset!`, and CI fails if it drifts from source (`git diff --exit-code -- dist/index.js`). Rebuild it and stage the regenerated `dist/index.js` alongside your source change:

```
cd crates/topcoat-runtime/browser
yarn install --frozen-lockfile
yarn build
yarn test
```

## New crates

A new crate must be references in the toplevel `Cargo.toml` as well as `release-plz.toml`.
