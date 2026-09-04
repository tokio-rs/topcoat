# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
