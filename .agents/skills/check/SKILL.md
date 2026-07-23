---
name: check
description: Always use this skill to verify a change locally before committing or opening a pull request in the Topcoat repository
---

# Verifying a Change

Load this skill after finishing a change and before committing or opening a
pull request. These commands mirror the jobs in
[`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml); running them
locally first keeps the PR from round-tripping through a red CI run.

CI builds with warnings denied (`RUSTFLAGS: -Dwarnings`,
`RUSTDOCFLAGS: -Dwarnings`), so a clippy lint or a rustdoc warning fails the
build. Treat every warning as an error.

## Format

```
cargo fmt --all
topcoat fmt
```

`cargo fmt` formats Rust. `topcoat fmt` additionally formats Topcoat macro
bodies such as `view!` (see
[`crates/topcoat-cli/docs/fmt.md`](../../../crates/topcoat-cli/docs/fmt.md)).
CI checks formatting with `cargo fmt --all -- --check`.

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

CI lints each feature in isolation to catch feature combinations that do not
build. This needs `cargo-hack`:

```
cargo hack clippy --workspace --each-feature --exclude-features stage-icons --no-dev-deps -- -D warnings
```

## Unused dependencies

CI fails on unused dependencies. This needs `cargo-udeps` on nightly:

```
cargo +nightly udeps --workspace --all-targets --all-features --locked
```

## Runtime browser bundle

Only when you touched `crates/topcoat-runtime/browser`. The runtime crate serves
a prebuilt `dist/index.js` via `asset!`, and CI fails if it drifts from source
(`git diff --exit-code -- dist/index.js`). Rebuild and commit it:

```
cd crates/topcoat-runtime/browser
yarn install --frozen-lockfile
yarn build
yarn test
```

Then stage the regenerated `dist/index.js` alongside your source change.

## Safety

This project uses only safe code; `unsafe` is not allowed
([`AGENTS.md`](../../../AGENTS.md)).
