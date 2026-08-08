# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-core-v0.5.0...topcoat-core-v0.6.0) - 2026-08-08

### Added

- *(core)* [**breaking**] make Cx detachable, remove CxBuilder ([#322](https://github.com/tokio-rs/topcoat/pull/322))
- *(core)* [**breaking**] specify as_ref manually on memoized functions ([#310](https://github.com/tokio-rs/topcoat/pull/310))
- use #[track_caller] where appropriate ([#262](https://github.com/tokio-rs/topcoat/pull/262))

### Fixed

- *(core)* detect recursive memoized calls ([#278](https://github.com/tokio-rs/topcoat/pull/278))
- *(core)* keep macro bodies intact when formatting rust snippets ([#273](https://github.com/tokio-rs/topcoat/pull/273))

### Other

- sort imports

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-core-v0.4.0...topcoat-core-v0.5.0) - 2026-07-27

### Added

- *(router)* server-sent events ([#218](https://github.com/tokio-rs/topcoat/pull/218))
- *(mail)* email prototype ([#216](https://github.com/tokio-rs/topcoat/pull/216))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-core-v0.1.3...topcoat-core-v0.2.0) - 2026-07-19

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.1.2](https://github.com/tokio-rs/topcoat/compare/topcoat-core-v0.1.1...topcoat-core-v0.1.2) - 2026-07-17

### Added

- add optional context accessors ([#115](https://github.com/tokio-rs/topcoat/pull/115))

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-core-v0.0.1) - 2026-07-14

### Other

- initial release
