---
name: pr
description: Always use this skill before opening a pull request in the Topcoat repository
---

# Opening Pull Requests

## Describe the diff, not the latest commit

A branch usually holds several commits: initial work, fixups, review responses, rebases. The title and body describe the net change landing on the base (usually `main`). Read the full diff first:

```
git diff <base>...HEAD
git log <base>..HEAD
```

## Title

Same Conventional Commits format as a commit message (see the [`commit`](../commit/SKILL.md) skill). [`.github/workflows/semantic-pr.yml`](../../../.github/workflows/semantic-pr.yml) enforces that the type is one of `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert` and that the subject is not capitalized. PRs are squash-merged, so the title becomes the landed commit and the `release-plz` changelog entry.

## Body

No template. Keep it short and high-signal:

- **Summary** -- what the change does and why, drawn from the diff.
- **Testing** -- how you verified it (which checks you ran; see the [`check`](../check/SKILL.md) skill).
- **Disclaimer** -- if you are an AI agent creating the PR, add a disclaimer about specific model and how AI was used to create the change. 

Reviewers already know Topcoat and Rust. Include what they need to evaluate the change, and nothing else.
