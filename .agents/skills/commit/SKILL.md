---
name: commit
description: Always use this skill before authoring a commit message in the Topcoat repository
---

# Authoring Commit Messages

Topcoat follows [Conventional Commits](https://www.conventionalcommits.org/). [`.github/workflows/semantic-pr.yml`](../../../.github/workflows/semantic-pr.yml) enforces the same format on PR titles, and PRs are squash-merged, so keep commits and PR titles consistent. `release-plz` reads the history to generate changelogs and pick version bumps (`feat` -> minor, `fix` -> patch, breaking -> major), so pick the type deliberately.

## Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

Only the header is required.

**Type** -- one of `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`. Only `feat` and `fix` are user-facing and appear in the changelog.

**Scope** -- optional. The area touched: a crate name with the `topcoat-` prefix dropped (`view`, `router`, `runtime`, `asset`, `cookie`, `session`, `tailwind`, `cli`, `core`, `font`, `icon`, `ui`, `htmx`, `alpine-ajax`), or a finer subsystem (macro, module) when clearer. Omit it when the change spans crates or the type says enough (`chore: bump dependencies`).

**Subject** -- imperative present tense ("add", not "added"/"adds"), lowercase first letter, no trailing period, short.

**Body** -- optional; add one when the "what" or the "why" is not obvious from the subject. Same tense. State the motivation and contrast it with the previous behavior.

**Footer** -- reference closed issues (`Closes #123`). For a breaking change, add `!` after the type or scope (`feat(router)!: ...`) and end with:

```
BREAKING CHANGE: <what breaks and how to migrate>
```

## Characters

Plain ASCII only, per the [`style`](../style/SKILL.md) skill: `-`/`--` not an em dash, `->` not a Unicode arrow, `...` not an ellipsis.

## Be succinct

Maintainers already know Topcoat and Rust. State what changed and why; skip restated context and throat-clearing.
