---
name: prose
description: Always use this skill before writing long form markdown documentation for Topcoat.
---

# Prose

## Placement

Put a markdown file into a `docs/` folder for the crate it affects, then link it via `#[doc = include_str!("../docs/file.md")]`. General guides for a feature should go into `crates/topcoat/docs` and then document the re-export module in `src/`.

## Structure

When writing a guide, start with a very simple summary of what the guide is about, potentially linking to resources (e.g. Tailwind website). Then carefully introduce basic usage before moving on to more advanced topics.

## General

* Use simple, concise language, no fancy words.
* Avoid exhaustively listig specific implementations or uses that could evolve over time and go stale.
* Use only ASCII characters in both code and documentation, e.g. `->` instead of unicode arrow or `...` instead of ellipsis character.
* Avoid em-dashes entirely. Use colons and semicolons sparingly.
