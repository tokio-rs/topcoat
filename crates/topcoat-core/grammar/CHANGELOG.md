# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0](https://github.com/tokio-rs/topcoat/compare/topcoat-core-grammar-v0.6.2...topcoat-core-grammar-v0.7.0) - 2026-09-04

### Added

- *(core)* add pretty printing impls for all `syn` types, remove `prettyplease` ([#372](https://github.com/tokio-rs/topcoat/pull/372))

### Fixed

- *(core)* memoize + borrowed async arg fails with AsyncFnOnce lifetime error ([#371](https://github.com/tokio-rs/topcoat/pull/371))

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-core-grammar-v0.5.0...topcoat-core-grammar-v0.6.0) - 2026-08-17

### Added

- *(core)* use 128-bit hash instead of Clone and Eq for memoization ([#337](https://github.com/tokio-rs/topcoat/pull/337))
- *(core)* [**breaking**] specify as_ref manually on memoized functions ([#310](https://github.com/tokio-rs/topcoat/pull/310))

### Fixed

- *(core)* keep macro bodies intact when formatting rust snippets ([#273](https://github.com/tokio-rs/topcoat/pull/273))

### Other

- *(view)* add promoted str optimization to avoid allocations ([#330](https://github.com/tokio-rs/topcoat/pull/330))
- sort imports

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-core-grammar-v0.4.0...topcoat-core-grammar-v0.5.0) - 2026-07-27

### Added

- *(mail)* email prototype ([#216](https://github.com/tokio-rs/topcoat/pull/216))

## [0.4.0](https://github.com/tokio-rs/topcoat/compare/topcoat-core-grammar-v0.3.1...topcoat-core-grammar-v0.4.0) - 2026-07-22

### Fixed

- pretty printer forcing newline in empty html elements ([#161](https://github.com/tokio-rs/topcoat/pull/161))

## [0.3.0](https://github.com/tokio-rs/topcoat/compare/topcoat-core-grammar-v0.2.0...topcoat-core-grammar-v0.3.0) - 2026-07-19

### Fixed

- view macro using ExprLet instead of Local ([#142](https://github.com/tokio-rs/topcoat/pull/142))
- rust-analzyer experimental diagnostics ([#135](https://github.com/tokio-rs/topcoat/pull/135))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-core-grammar-v0.1.3...topcoat-core-grammar-v0.2.0) - 2026-07-19

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.1.1](https://github.com/tokio-rs/topcoat/compare/topcoat-core-grammar-v0.1.0...topcoat-core-grammar-v0.1.1) - 2026-07-16

### Added

- sessions ([#109](https://github.com/tokio-rs/topcoat/pull/109))

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-core-grammar-v0.0.1) - 2026-07-14

### Other

- initial release
