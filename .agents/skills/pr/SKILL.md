---
name: pr
description: Always use this skill before opening a pull request in the Topcoat repository
---

# Opening Pull Requests

Load this skill before opening a pull request in this project.

## Base the title and body on the diff, not the latest commit

A branch usually contains several commits: initial work, fixups, review responses, rebases. The title and body describe the net change that will land on the base branch, not the most recent commit. Read the full diff first:

```
git diff <base>...HEAD
git log <base>..HEAD
```

The base is usually `main`. Draft the title and body from what that diff actually contains.

## Title

The title follows the same Conventional Commits format as a commit message (see the [`commit`](../commit/SKILL.md) skill). It is checked by [`.github/workflows/semantic-pr.yml`](../../../.github/workflows/semantic-pr.yml) and, because PRs are squash-merged, it becomes the landed commit and the `release-plz` changelog entry. Two rules the check enforces:

- the type must be one of `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`;
- the subject must not start with an uppercase letter.

## Body

Topcoat has no pull request template. Keep the body short and high-signal:

- **Summary** -- what the change does and why, drawn from the diff.
- **Testing** -- how you verified it (which checks you ran; see below).

State anything a reviewer needs in order to evaluate the change, and nothing they do not. Reviewers already know Topcoat and Rust, so skip restated context and obvious explanation. A reviewer should grasp the important bits in seconds.

