---
name: prose
description: Always use this skill before writing long form markdown documentation for Topcoat.
---

# Prose

## Placement

Put guide sources in the top-level `docs/` directory, following the module structure. For example, use `docs/router.md`, `docs/router/module.md`, and `docs/router/content/sse.md`. Crate-local `docs/` files must be relative symlinks to those sources, with the crate module prefix omitted. The facade crate keeps the full module paths. Embed documentation through the crate-local path with `#[doc = include_str!("../docs/file.md")]` so published crates include it. Edit the top-level source and link to it from `README.md` and `AGENTS.md` where appropriate.

## Structure

When writing a guide, start with a very simple summary of what the guide is about, potentially linking to resources (e.g. Tailwind website). Then carefully introduce basic usage before moving on to more advanced topics.

When a feature is already best explained in detail by another part of the documentation (e.g. another markdown file), explain it at most briefly and then refer the reader to the related docs file via a Rust docs link.

## General

* Write in plain english. No fancy sentence structure.
* Avoid exhaustively listing specific implementations or uses that could evolve over time and go stale.
* Use only ASCII characters in both code and documentation, e.g. `->` instead of unicode arrow or `...` instead of ellipsis character.
* Avoid em-dashes entirely. Use colons and semicolons sparingly.
* Keep individual paragraphs in a markdown file on a single line.
