# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-macro-v0.6.2...topcoat-runtime-macro-v0.7.0) - 2026-09-04

### Added

- [**breaking**] streaming SSR and `live!` + `emit!` regions ([#373](https://github.com/tokio-rs/topcoat/pull/373))
- *(core)* add pretty printing impls for all `syn` types, remove `prettyplease` ([#372](https://github.com/tokio-rs/topcoat/pull/372))

### Fixed

- *(runtime)* preserve boolean procedure results ([#375](https://github.com/tokio-rs/topcoat/pull/375))

## [0.6.1](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-macro-v0.6.0...topcoat-runtime-macro-v0.6.1) - 2026-08-18

### Fixed

- *(runtime)* support await expressions in ExprBlock and ExprIf ([#352](https://github.com/tokio-rs/topcoat/pull/352))

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-macro-v0.5.0...topcoat-runtime-macro-v0.6.0) - 2026-08-17

### Added

- *(view)* improve new arena rendering system ([#319](https://github.com/tokio-rs/topcoat/pull/319))
- *(view)* concurrent rendering ([#317](https://github.com/tokio-rs/topcoat/pull/317))

### Fixed

- *(runtime)* let push_str accept an owned string surrogate ([#269](https://github.com/tokio-rs/topcoat/pull/269))
- *(runtime)* render f64 text the way Rust's Display does ([#245](https://github.com/tokio-rs/topcoat/pull/245))
- *(runtime)* match Rust semantics for string comparison and trim ([#244](https://github.com/tokio-rs/topcoat/pull/244))

### Other

- *(runtime)* note that page guards do not cover shard endpoints ([#251](https://github.com/tokio-rs/topcoat/pull/251))

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-macro-v0.4.0...topcoat-runtime-macro-v0.5.0) - 2026-07-27

### Added

- *(runtime)* add utility methods for signals with primitive types ([#214](https://github.com/tokio-rs/topcoat/pull/214))

### Other

- fix outdated documentation ([#211](https://github.com/tokio-rs/topcoat/pull/211))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-macro-v0.1.3...topcoat-runtime-macro-v0.2.0) - 2026-07-19

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.1.3](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-macro-v0.1.2...topcoat-runtime-macro-v0.1.3) - 2026-07-17

### Fixed

- fix udeps

### Other

- add reactivity guide ([#123](https://github.com/tokio-rs/topcoat/pull/123))

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-runtime-macro-v0.0.1) - 2026-07-14

### Other

- initial release
