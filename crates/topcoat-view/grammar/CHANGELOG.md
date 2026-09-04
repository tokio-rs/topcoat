# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0](https://github.com/tokio-rs/topcoat/compare/topcoat-view-grammar-v0.6.2...topcoat-view-grammar-v0.7.0) - 2026-09-04

### Added

- [**breaking**] streaming SSR and `live!` + `emit!` regions ([#373](https://github.com/tokio-rs/topcoat/pull/373))

### Other

- upgrade to rust 1.98 ([#367](https://github.com/tokio-rs/topcoat/pull/367))

## [0.6.2](https://github.com/tokio-rs/topcoat/compare/topcoat-view-grammar-v0.6.1...topcoat-view-grammar-v0.6.2) - 2026-08-18

### Fixed

- *(view)* make control-flow futures own their pattern bindings ([#360](https://github.com/tokio-rs/topcoat/pull/360))

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-view-grammar-v0.5.0...topcoat-view-grammar-v0.6.0) - 2026-08-17

### Added

- *(core)* stable component identity system ([#328](https://github.com/tokio-rs/topcoat/pull/328))
- *(view)* improve new arena rendering system ([#319](https://github.com/tokio-rs/topcoat/pull/319))
- *(view)* concurrent rendering ([#317](https://github.com/tokio-rs/topcoat/pull/317))

### Fixed

- *(core)* keep macro bodies intact when formatting rust snippets ([#273](https://github.com/tokio-rs/topcoat/pull/273))
- *(view)* allow keyword element names ([#274](https://github.com/tokio-rs/topcoat/pull/274))

### Other

- *(view)* add promoted str optimization to avoid allocations ([#330](https://github.com/tokio-rs/topcoat/pull/330))
- *(view)* refactor new rendering system part 2 ([#320](https://github.com/tokio-rs/topcoat/pull/320))
- *(view)* add lowering step to high-level intermediate representation ([#316](https://github.com/tokio-rs/topcoat/pull/316))
- sort imports

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-view-grammar-v0.4.0...topcoat-view-grammar-v0.5.0) - 2026-07-27

### Added

- boxed components for cyclic component definition ([#176](https://github.com/tokio-rs/topcoat/pull/176))

### Fixed

- topcoat formatter breaking at signal syntax ([#172](https://github.com/tokio-rs/topcoat/pull/172))

## [0.4.0](https://github.com/tokio-rs/topcoat/compare/topcoat-view-grammar-v0.3.1...topcoat-view-grammar-v0.4.0) - 2026-07-22

### Added

- add Alpine AJAX integration and example ([#158](https://github.com/tokio-rs/topcoat/pull/158))

### Fixed

- pretty printer forcing newline in empty html elements ([#161](https://github.com/tokio-rs/topcoat/pull/161))

## [0.3.0](https://github.com/tokio-rs/topcoat/compare/topcoat-view-grammar-v0.2.0...topcoat-view-grammar-v0.3.0) - 2026-07-19

### Fixed

- view macro using ExprLet instead of Local ([#142](https://github.com/tokio-rs/topcoat/pull/142))
- rust-analzyer experimental diagnostics ([#135](https://github.com/tokio-rs/topcoat/pull/135))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-view-grammar-v0.1.3...topcoat-view-grammar-v0.2.0) - 2026-07-19

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.1.3](https://github.com/tokio-rs/topcoat/compare/topcoat-view-grammar-v0.1.2...topcoat-view-grammar-v0.1.3) - 2026-07-17

### Fixed

- shards no longer need cx =>

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-view-grammar-v0.0.1) - 2026-07-14

### Other

- initial release
