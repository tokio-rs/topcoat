---
name: check
description: Always use this skill to verify a change locally before committing or opening a pull request in the Topcoat repository
---

# Verifying a Change

Keep this file in sync with CI workflows.

## Format

```
cargo +nightly fmt --all
cargo topcoat fmt
```

If the user does not have the nightly formatter installed, that is fine. Use the stable formatter. `topcoat fmt` formats the Topcoat macros inside of the soruce files.

## Lint

```
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

## Test

```
cargo test --workspace --all-features
```

## Docs

Rustdoc builds on nightly with the `docsrs` cfg:

```
RUSTDOCFLAGS="--cfg docsrs -Dwarnings" cargo +nightly doc --workspace --all-features --no-deps --locked
```

## Per-feature lint

CI lints each feature in isolation to catch feature combinations that do not build. This needs `cargo-hack`:

```
cargo hack clippy --workspace --each-feature --exclude-features stage-icons --no-dev-deps -- -D warnings
```

Only run on user request.

## Unused dependencies

CI fails on unused dependencies. This needs `cargo-udeps` on nightly:

```
cargo +nightly udeps --workspace --all-targets --all-features --locked
```

Only run on user request.

## Runtime browser bundle

Only when you touched `crates/topcoat-runtime/browser`. The runtime crate serves a prebuilt `dist/index.js` via `asset!`, and CI fails if it drifts from source (`git diff --exit-code -- dist/index.js`). Rebuild and commit it:

```
cd crates/topcoat-runtime/browser
yarn install --frozen-lockfile
yarn build
yarn test
```

Then stage the regenerated `dist/index.js` alongside your source change.

## Safety

This project uses only safe code; `unsafe` is not allowed ([`AGENTS.md`](../../../AGENTS.md)).
