# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.12.0](https://github.com/tokio-rs/topcoat/compare/topcoat-mail-v0.11.0...topcoat-mail-v0.12.0) - 2026-11-09

### Added

- [**breaking**] remove `mail!` and `cookie!` macros ([#585](https://github.com/tokio-rs/topcoat/pull/585))
- [**breaking**] unify macro field parsing and use explicit mail views ([#583](https://github.com/tokio-rs/topcoat/pull/583))
- [**breaking**] require leading `cx: &Cx` and capture context in view macros ([#581](https://github.com/tokio-rs/topcoat/pull/581))
- [**breaking**] lenient macros ([#580](https://github.com/tokio-rs/topcoat/pull/580))
- *(router)* route context with `#[route(with(...))]` ([#566](https://github.com/tokio-rs/topcoat/pull/566))
- *(core)* [**breaking**] unify `app_context` and `request_context` into one `context` ([#562](https://github.com/tokio-rs/topcoat/pull/562))

### Other

- [**breaking**] move more things into request context ([#593](https://github.com/tokio-rs/topcoat/pull/593))
- *(mail)* [**breaking**] simplify mail crate ([#592](https://github.com/tokio-rs/topcoat/pull/592))
- *(mail)* drop unused topcoat-mail dependency from the grammar crate

## [0.10.0](https://github.com/tokio-rs/topcoat/compare/topcoat-mail-v0.9.0...topcoat-mail-v0.10.0) - 2026-10-03

### Fixed

- non-deterministic macros ([#485](https://github.com/tokio-rs/topcoat/pull/485))
- *(cli)* `cargo topcoat` not working ([#480](https://github.com/tokio-rs/topcoat/pull/480))

### Other

- restructure repository documentation ([#454](https://github.com/tokio-rs/topcoat/pull/454))

## [0.9.0](https://github.com/tokio-rs/topcoat/compare/topcoat-mail-v0.8.1...topcoat-mail-v0.9.0) - 2026-09-24

### Other

- rewrite all docs 3 ([#439](https://github.com/tokio-rs/topcoat/pull/439))
- rewrite all docs ([#435](https://github.com/tokio-rs/topcoat/pull/435))

## [0.7.0](https://github.com/tokio-rs/topcoat/compare/topcoat-mail-v0.6.2...topcoat-mail-v0.7.0) - 2026-09-04

### Added

- [**breaking**] streaming SSR and `live!` + `emit!` regions ([#373](https://github.com/tokio-rs/topcoat/pull/373))

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-mail-v0.5.0...topcoat-mail-v0.6.0) - 2026-08-17

### Added

- *(view)* improve new arena rendering system ([#319](https://github.com/tokio-rs/topcoat/pull/319))
- *(view)* concurrent rendering ([#317](https://github.com/tokio-rs/topcoat/pull/317))
- *(router)* [**breaking**] add unused layer sanity check ([#300](https://github.com/tokio-rs/topcoat/pull/300))
- use #[track_caller] where appropriate ([#262](https://github.com/tokio-rs/topcoat/pull/262))

### Other

- *(router)* [**breaking**] replace handler structs with traits in preparation for href ([#346](https://github.com/tokio-rs/topcoat/pull/346))
- *(view)* consume view when rendering to improve performance ([#312](https://github.com/tokio-rs/topcoat/pull/312))
- sort imports
- make request and response dedicated modules
