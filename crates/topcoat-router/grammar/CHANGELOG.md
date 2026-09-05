# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-grammar-v0.7.0...topcoat-router-grammar-v0.8.0) - 2026-09-05

### Added

- *(runtime)* [**breaking**] replace signal custom syntax with an ordinary rust function ([#384](https://github.com/tokio-rs/topcoat/pull/384))

## [0.7.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-grammar-v0.6.2...topcoat-router-grammar-v0.7.0) - 2026-09-04

### Added

- [**breaking**] streaming SSR and `live!` + `emit!` regions ([#373](https://github.com/tokio-rs/topcoat/pull/373))

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-grammar-v0.5.0...topcoat-router-grammar-v0.6.0) - 2026-08-17

### Added

- *(router)* `href!` macro ([#350](https://github.com/tokio-rs/topcoat/pull/350))
- *(core)* [**breaking**] specify as_ref manually on memoized functions ([#310](https://github.com/tokio-rs/topcoat/pull/310))
- *(router)* [**breaking**] add not_found macro and no longer run layers and layouts by default on unmatched requests ([#298](https://github.com/tokio-rs/topcoat/pull/298))
- use #[track_caller] where appropriate ([#262](https://github.com/tokio-rs/topcoat/pull/262))
- *(router)* [**breaking**] replace path parameter attribute macro ([#242](https://github.com/tokio-rs/topcoat/pull/242))

### Fixed

- *(router)* reject invalid route signatures at parse time ([#241](https://github.com/tokio-rs/topcoat/pull/241))
- include docs and visibility in routes, procedures, layers ([#232](https://github.com/tokio-rs/topcoat/pull/232))

### Other

- *(router)* [**breaking**] replace handler structs with traits in preparation for href ([#346](https://github.com/tokio-rs/topcoat/pull/346))
- *(router)* rename erased constant for more readable profiler and debugger traces ([#329](https://github.com/tokio-rs/topcoat/pull/329))
- sort imports
- make request and response dedicated modules

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-grammar-v0.4.0...topcoat-router-grammar-v0.5.0) - 2026-07-27

### Added

- make page HTTP methods customizable ([#181](https://github.com/tokio-rs/topcoat/pull/181))
- add support for routes that handle multiple (or all) methods ([#180](https://github.com/tokio-rs/topcoat/pull/180))

### Other

- [**breaking**] dedicated router error module ([#183](https://github.com/tokio-rs/topcoat/pull/183))
- [**breaking**] pass layouts the rendered Result<View> instead of a Slot future ([#166](https://github.com/tokio-rs/topcoat/pull/166))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-grammar-v0.1.3...topcoat-router-grammar-v0.2.0) - 2026-07-19

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-router-grammar-v0.0.1) - 2026-07-14

### Other

- initial release
