# Contributing to Topcoat

This guide explains how to prepare and submit a change to Topcoat.

## Discussing a change

Open a pull request directly for a bug fix or documentation improvement.

If you want to build a feature, please talk to us in the [Tokio Discord](https://discord.gg/tokio) before you write the code.

Discussing a feature first lets us check whether it fits the design and can be maintained before you spend time implementing it.

AI-assisted contributions are welcome. Read the affected code and review the full diff before submitting it. You are responsible for understanding and explaining the change.

Keep pull requests focused. Discuss the scope if a fix grows into a feature.

## Local setup

Topcoat is a plain Cargo workspace. The framework crates live in `crates/`, small single-feature examples in `examples/`, and complete demo applications in `demos/`.

Use the toolchain selected by `rust-toolchain.toml` to build and test. Formatting and the documentation check also require nightly:

```sh
git clone https://github.com/tokio-rs/topcoat
cd topcoat
cargo test --workspace --all-features
```

If you use Nix, `nix develop` gives you a shell with the toolchain already set up.

Install the CLI from the workspace so `topcoat fmt` and the dev server match the code you are working on:

```sh
cargo install --path crates/topcoat-cli
```

Run `topcoat dev` from an example directory to try the feature you are changing. Use a demo to check how it behaves in a larger app.

## Fork and branch

1. Fork the repository and clone your fork.
2. Branch off `main`.
3. Make your change, with tests where it makes sense.
4. Run the checks below.
5. Push to your fork and open a pull request against `main`.

Pull requests are squash-merged, so you do not need to tidy up your commit history before pushing. Keep your branch mergeable with `main` by rebasing rather than merging `main` into it.

## Formatting, linting, and testing

Run these checks before opening a pull request:

```sh
cargo +nightly fmt --all # nightly is required, CI checks formatting with it
cargo topcoat fmt # formats Topcoat macro bodies inside source files
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="--cfg docsrs -Dwarnings" cargo +nightly doc --workspace --all-features --no-deps --locked
```

CI runs a few more jobs that are slower to reproduce locally. Run them if your change is likely to affect them:

```sh
# feature combinations that do not build (needs cargo-hack)
cargo hack clippy --workspace --each-feature --exclude-features stage-icons --no-dev-deps -- -D warnings

# unused dependencies (needs cargo-udeps on nightly)
cargo +nightly udeps --workspace --all-targets --all-features --locked
```

For browser changes, run the following commands from `crates/topcoat-runtime/browser` or `crates/topcoat-cli/browser`, depending on the project you changed. Changes to the shared code in `crates/topcoat-core/browser` need both projects checked and rebuilt. CI checks formatting, linting, tests, and that the committed bundles match their source:

```sh
yarn install --frozen-lockfile
yarn lint
yarn build
yarn test
```

Commit the regenerated `dist/index.js` with the source changes. The runtime also builds `dist/coherence.js`, which must be committed when it changes.

The full check list, including when each command is needed, is in the [`check`](.agents/skills/check/SKILL.md) skill.

## Code and documentation style

Follow these conventions:

- Plain ASCII everywhere, in code, docs, and commit messages. Write `->` instead of an arrow character and `...` instead of an ellipsis, and avoid em dashes.
- Do not write unsafe code.

Follow the [`style`](.agents/skills/style/SKILL.md) skill for code and documentation, the [`prose`](.agents/skills/prose/SKILL.md) skill for guides, and the [`macro`](.agents/skills/macro/SKILL.md) skill when writing procedural macros.

## Commits and pull requests

Commit messages and pull request titles follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>
```

Choose the type using the [`commit`](.agents/skills/commit/SKILL.md) skill. The optional scope names the affected area, usually the crate name without `topcoat-`. Write a lowercase imperative subject with no trailing period:

```
fix(router): isolate request panics
```

Pull requests are squash-merged, so the title becomes the commit subject and affects the release version. Mark breaking changes with `!` after the type or scope, and add a `BREAKING CHANGE:` footer with migration instructions.

Describe the problem, the resulting behavior, and how you verified the change. Reference any issue it closes with `Closes #123`. If you used an AI agent, name the model and explain what it did. Keep the description concise.

More detail is in the [`commit`](.agents/skills/commit/SKILL.md) and [`pr`](.agents/skills/pr/SKILL.md) skills.

## Where to find things

- [README](README.md): an introduction and links to the documentation.
- [`crates/topcoat/docs/getting_started.md`](crates/topcoat/docs/getting_started.md): building an app with Topcoat, which is worth doing before changing the framework.
- [Tokio Discord](https://discord.gg/tokio): questions, feature discussions, and everything else.

By contributing, you agree that your contributions are licensed under the [MIT license](LICENSE).
