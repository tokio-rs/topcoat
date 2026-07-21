---
name: commit
description: Always use this skill before authoring a commit message in the Topcoat repository
---

# Authoring Commit Messages

Load this skill before writing any git commit message in this project.

Topcoat follows [Conventional Commits](https://www.conventionalcommits.org/).
The same format is enforced on pull request titles by
[`.github/workflows/semantic-pr.yml`](../../../.github/workflows/semantic-pr.yml),
and PRs are squash-merged, so the PR title becomes the landed commit. Commit
messages and PR titles use one vocabulary; keep them consistent.

Commit messages also feed release automation: `release-plz` reads the
Conventional Commits history to generate each crate's changelog and pick the
version bump. The type you choose has a visible effect (`feat` -> minor,
`fix` -> patch, a breaking change -> major), so pick it deliberately.

## Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

Only the header is required.

## Type

One of the types accepted by the semantic-pr check:

`feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`,
`chore`, `revert`.

`feat` and `fix` are user-facing and show up in the changelog. `docs`, `style`
(formatting, no behavior change), `refactor`, `perf`, `test`, `build`, `ci`,
and `chore` cover everything else. `revert` reverts an earlier commit.

## Scope

The scope is optional (the semantic-pr check sets `requireScope: false`). When
present, name the area being touched. For a single crate, use its name with the
`topcoat-` prefix dropped, for example `view`, `router`, `runtime`, `asset`,
`cookie`, `session`, `tailwind`, `cli`, `core`, `font`, `icon`, `ui`, `htmx`, or
`alpine-ajax`. A finer subsystem name (such as a macro or a module) is also fine
when it is clearer. Omit the scope when a change spans several crates or the type alone
already says enough (`chore: bump dependencies`).

## Subject

- imperative, present tense: "add" not "added" nor "adds"
- lowercase first letter (the semantic-pr check rejects an uppercase subject on
  PR titles; keep commits consistent)
- no period at the end
- keep it short; move detail to the body

## Body

Optional. Add one when the "what" is not obvious from the subject, or the "why"
would not be obvious to a future reader. Use the same imperative present tense.
State the motivation and contrast it with the previous behavior.

## Footer

Reference issues the commit closes (`Closes #123`). For a breaking change, add
a `!` after the type or scope (`feat(router)!: ...`) and end the message with a
paragraph describing what breaks and how to migrate:

```
BREAKING CHANGE: <what breaks and how to migrate>
```

`release-plz` turns this into a major version bump.

## Characters

Write messages with plain-ASCII characters only, per
[`STYLE.md`](../../../STYLE.md): use `-` or `--` instead of an em dash, `->`
instead of a Unicode arrow, and `...` instead of an ellipsis.

## Be succinct

Maintainers reading the log already know Topcoat and Rust. State what changed
and why; skip restated context and throat-clearing. A maintainer should grasp
the important bits in seconds.
