---
name: prose
description: Always use this skill before writing long form markdown documentation for Topcoat.
---

# Prose

## Placement

Put a markdown file into a `docs/` folder for the crate it affects, then link it via `#[doc = include_str!("../docs/file.md")]`. General guides for a feature should go into `crates/topcoat/docs` and then document the re-export module in `src/`. These should likely also be referenced in the `README.md` and `AGENTS.md`.

## Structure

When writing a guide, start with a very simple summary of what the guide is about, potentially linking to resources (e.g. Tailwind website). Then carefully introduce basic usage before moving on to more advanced topics.

When a feature is already best explained in detail by another part of the documentation (e.g. another markdown file), explain it at most briefly and then refer the reader to the related docs file via a Rust docs link.

## General

* Use simple, concise language, no fancy words.
* Avoid exhaustively listig specific implementations or uses that could evolve over time and go stale.
* Use only ASCII characters in both code and documentation, e.g. `->` instead of unicode arrow or `...` instead of ellipsis character.
* Avoid em-dashes entirely. Use colons and semicolons sparingly.
* Keep individual paragraphs in a markdown file on a single line.
