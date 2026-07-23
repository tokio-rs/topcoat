---
name: pr
description: Always use this skill before opening a pull request in the Topcoat repository
---

# Opening Pull Requests

Load this skill before opening a pull request in this project.

## Base the title and body on the diff, not the latest commit

A branch usually contains several commits: initial work, fixups, review
responses, rebases. The title and body describe the net change that will land
on the base branch, not the most recent commit. Read the full diff first:

```
git diff <base>...HEAD
git log <base>..HEAD
```

The base is usually `main`. Draft the title and body from what that diff
actually contains.

## Title

The title follows the same Conventional Commits format as a commit message (see
the [`commit`](../commit/SKILL.md) skill). It is checked by
[`.github/workflows/semantic-pr.yml`](../../../.github/workflows/semantic-pr.yml)
and, because PRs are squash-merged, it becomes the landed commit and the
`release-plz` changelog entry. Two rules the check enforces:

- the type must be one of `feat`, `fix`, `docs`, `style`, `refactor`, `perf`,
  `test`, `build`, `ci`, `chore`, `revert`;
- the subject must not start with an uppercase letter.

## Body

Topcoat has no pull request template. Keep the body short and high-signal:

- **Summary** -- what the change does and why, drawn from the diff.
- **Testing** -- how you verified it (which checks you ran; see below).

State anything a reviewer needs in order to evaluate the change, and nothing
they do not. Reviewers already know Topcoat and Rust, so skip restated context
and obvious explanation. A reviewer should grasp the important bits in seconds.

## Run the checks first

Before opening the PR, run the local checks that mirror CI so the PR lands
green. See the [`check`](../check/SKILL.md) skill for the exact commands. CI
denies warnings everywhere, so a clippy or rustdoc warning fails the build.

## Markdown-doc PRs: link the rendered version

When a PR's primary change is adding or substantially rewriting a single
markdown file (a guide page under `docs/` or a crate's `docs/`), put a link to
the rendered version on the branch as the first line under `## Summary`:

```
## Summary

[Rendered](https://github.com/tokio-rs/topcoat/blob/<branch-name>/<path-to-file>.md)

<rest of summary>
```

Use the PR's head branch name so the link renders the version under review, not
what is on `main`.

## Labels

Do not apply labels when opening the PR. The `release` label is reserved for
`release-plz`'s automated release PRs
([`release-plz.toml`](../../../release-plz.toml)); maintainers triage and label
everything else. Passing `--label` to `gh pr create` bypasses that.

## Characters

Write the title and body with plain-ASCII characters only, per
[`STYLE.md`](../../../STYLE.md): `-` or `--` for an em dash, `->` for a Unicode
arrow, `...` for an ellipsis.
