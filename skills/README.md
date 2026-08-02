# Topcoat skills

This directory contains agent skills for building applications with [Topcoat](https://github.com/tokio-rs/topcoat). A skill is a reusable set of instructions in a `SKILL.md` file that gives a coding agent framework-specific context and working conventions.

The skills are intended for application development. The repository's `AGENTS.md` and `.agents/skills/` still contain contributor instructions for changing Topcoat itself.

## Available skills

- `topcoat`: The framework's core mental model, setup, and common application workflow.
- `topcoat-routing`: Route trees, pages, layouts, layers, HTTP inputs and responses, errors, and advanced transports.
- `topcoat-ui`: Views, components, CSS, Tailwind, Topcoat UI, assets, fonts, icons, and accessibility.
- `topcoat-runtime`: Signals, expressions, event handlers, binds, procedures, shards, and client/server boundaries.
- `topcoat-auth`: Cookies, sessions, request helpers, origin checks, and authorization.
- `topcoat-testing`: Tests for views, routes, context, sessions, runtime behavior, and assets.
- `topcoat-data`: Database clients, data helpers, mutations, transactions, and request-local caching.

## Install

Use the [skills CLI](https://github.com/vercel-labs/skills) to choose and install the application skills in this directory:

```sh
npx skills add tokio-rs/topcoat
```

Topcoat's contributor-only skills under `.agents/skills/` are marked internal, so this command only lists the application skills above.
