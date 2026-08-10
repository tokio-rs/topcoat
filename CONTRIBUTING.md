# Contributing to Topcoat

Thanks for your interest in Topcoat. This guide is the short, human-facing version of how a change gets from your machine into the repository.

## Fixes are welcome, features need a conversation first

If you found a bug and have a fix, or spotted a mistake, a broken link, or a confusing paragraph in the documentation, just open a pull request. You do not need to ask first, file an issue first, or wait for anyone.

If you want to build a feature, please talk to us in the [Tokio Discord](https://discord.gg/tokio) before you write the code.

Topcoat is early in its development and we have a clear picture of where we want it to go. That means we have to be critical of every feature that lands: it has to fit the design, earn the API surface it adds, and be something we can maintain for years. Plenty of otherwise reasonable proposals do not clear that bar, and finding that out after you already wrote the code is the worst outcome for everyone. A short conversation up front is cheap, a rejected weekend of work is not.

The same goes for AI-assisted contributions. Using an agent to help write a change is fine, we do it too. But treat the result as your own work: read the code it touches, read the diff it produced, and be ready to explain and defend every line. If nobody has read a change before it arrives, we could have prompted for it ourselves, and all that is left for us is the review. Reviewing is the expensive part, and it does not scale.

Small, focused pull requests are much easier to accept than large ones. If a change grows past a fix, that is usually the signal to open a conversation.

## Local setup

Topcoat is a plain Cargo workspace. The framework crates live in `crates/`, small single-feature examples in `examples/`, and complete demo applications in `demos/`.

A stable toolchain is enough to build and test everything. `rust-toolchain.toml` pins stable, and a nightly toolchain is required for the formatter and the doc check:

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

The examples are the fastest way to try a change against a real app. Each one is a workspace member, so `topcoat dev` inside `examples/hello-world` (or any other example directory) builds and serves it. An example covers one feature; a demo in `demos/` is a whole application that puts many of them together, which is the better place to see how a change holds up in context.

## Fork and branch

1. Fork the repository and clone your fork.
2. Branch off `main`.
3. Make your change, with tests where it makes sense.
4. Run the checks below.
5. Push to your fork and open a pull request against `main`.

Pull requests are squash-merged, so you do not need to tidy up your commit history before pushing. Keep your branch mergeable with `main` by rebasing rather than merging `main` into it.

## Formatting, linting, and testing

Run this before every pull request. It mirrors what CI does, so it saves you a round-trip:

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

If you touched `crates/topcoat-runtime/browser`, the prebuilt browser bundle has to be rebuilt and committed alongside your source change. CI fails if it drifts:

```sh
cd crates/topcoat-runtime/browser
yarn install --frozen-lockfile
yarn build
yarn test
```

The full check list, including when each command is needed, is in the [`check`](.agents/skills/check/SKILL.md) skill.

## Code and documentation style

Two conventions are worth knowing up front, because they show up in almost every diff:

- Plain ASCII everywhere, in code, docs, and commit messages. Write `->` instead of an arrow character and `...` instead of an ellipsis, and avoid em dashes.
- No `unsafe`, unless it comes from a reputable dependency.

Beyond that, match the code around you. The [`style`](.agents/skills/style/SKILL.md) skill covers the rest (module layout, dependency declarations, documentation wording), the [`prose`](.agents/skills/prose/SKILL.md) skill covers the guides in each crate's `docs/` directory, and the [`macro`](.agents/skills/macro/SKILL.md) skill covers the proc-macro crates.

## Commits and pull requests

Commit messages and pull request titles follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>
```

The type is one of `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. The optional scope is the area you touched, usually a crate name with the `topcoat-` prefix dropped (`view`, `router`, `runtime`, `cli`, ..). The subject is imperative present tense, lowercase, with no trailing period:

```
fix(router): isolate request panics
```

A CI job enforces this format on pull request titles, and since pull requests are squash-merged, the title becomes the landed commit and the changelog entry. `release-plz` derives version bumps from it, so pick the type deliberately: `feat` is a minor bump, `fix` a patch, and a `!` after the type or scope marks a breaking change, which also needs a `BREAKING CHANGE:` footer explaining how to migrate.

There is no pull request template. Describe the whole diff rather than your last commit, say how you verified it, and reference any issue it closes (`Closes #123`). If you used an AI agent, mention which model and what it did. Reviewers know Topcoat and Rust, so keep the description short and high-signal.

More detail is in the [`commit`](.agents/skills/commit/SKILL.md) and [`pr`](.agents/skills/pr/SKILL.md) skills.

## Where to find things

- [README](README.md): what Topcoat is, a guide index, and the roadmap.
- [`crates/topcoat/docs/getting_started.md`](crates/topcoat/docs/getting_started.md): building an app with Topcoat, which is worth doing before changing the framework.
- [Tokio Discord](https://discord.gg/tokio): questions, feature discussions, and everything else.

By contributing, you agree that your contributions are licensed under the [MIT license](LICENSE).
