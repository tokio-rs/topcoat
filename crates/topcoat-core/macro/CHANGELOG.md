# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-core-macro-v0.5.0...topcoat-core-macro-v0.6.0) - 2026-08-17

### Added

- *(core)* [**breaking**] add scoped context using `cx.with(...)` ([#338](https://github.com/tokio-rs/topcoat/pull/338))
- *(core)* use 128-bit hash instead of Clone and Eq for memoization ([#337](https://github.com/tokio-rs/topcoat/pull/337))
- *(core)* [**breaking**] specify as_ref manually on memoized functions ([#310](https://github.com/tokio-rs/topcoat/pull/310))

### Fixed

- *(core)* detect recursive memoized calls ([#278](https://github.com/tokio-rs/topcoat/pull/278))

### Other

- fix memoize docs stale example

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-core-macro-v0.4.0...topcoat-core-macro-v0.5.0) - 2026-07-27

### Other

- [**breaking**] pass layouts the rendered Result<View> instead of a Slot future ([#166](https://github.com/tokio-rs/topcoat/pull/166))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-core-macro-v0.1.3...topcoat-core-macro-v0.2.0) - 2026-07-19

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-core-macro-v0.0.1) - 2026-07-14

### Other

- initial release
