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

## Browser projects

For changes in `crates/topcoat-runtime/browser` or `crates/topcoat-cli/browser`, run these commands from the affected project's directory. For shared code in `crates/topcoat-core/browser`, run them in both projects. `yarn lint` checks formatting, imports, and lint rules, including the shared code.

```
yarn install --frozen-lockfile
yarn lint
yarn build
yarn test
```

The runtime and CLI embed their prebuilt `dist/index.js`; the runtime's coherence crate also embeds `dist/coherence.js`. CI rebuilds these files and rejects drift. Include regenerated bundles alongside source changes.

## New crates

A new crate must be referenced in the toplevel `Cargo.toml` as well as `release-plz.toml`.
