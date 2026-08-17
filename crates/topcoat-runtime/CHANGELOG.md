# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-v0.5.0...topcoat-runtime-v0.6.0) - 2026-08-17

### Added

- *(view)* improve new arena rendering system ([#319](https://github.com/tokio-rs/topcoat/pull/319))
- *(view)* concurrent rendering ([#317](https://github.com/tokio-rs/topcoat/pull/317))
- use #[track_caller] where appropriate ([#262](https://github.com/tokio-rs/topcoat/pull/262))

### Fixed

- *(runtime)* support signals outside the body ([#334](https://github.com/tokio-rs/topcoat/pull/334))
- *(runtime)* let push_str accept an owned string surrogate ([#269](https://github.com/tokio-rs/topcoat/pull/269))
- *(runtime)* reject non-2xx shard responses instead of rendering them ([#247](https://github.com/tokio-rs/topcoat/pull/247))
- *(runtime)* render f64 text the way Rust's Display does ([#245](https://github.com/tokio-rs/topcoat/pull/245))
- *(runtime)* match Rust semantics for string comparison and trim ([#244](https://github.com/tokio-rs/topcoat/pull/244))

### Other

- *(router)* [**breaking**] replace handler structs with traits in preparation for href ([#346](https://github.com/tokio-rs/topcoat/pull/346))
- *(view)* add promoted str optimization to avoid allocations ([#330](https://github.com/tokio-rs/topcoat/pull/330))
- *(router)* rename erased constant for more readable profiler and debugger traces ([#329](https://github.com/tokio-rs/topcoat/pull/329))
- sort imports
- make request and response dedicated modules
- *(runtime)* note that page guards do not cover shard endpoints ([#251](https://github.com/tokio-rs/topcoat/pull/251))

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-v0.4.0...topcoat-runtime-v0.5.0) - 2026-07-27

### Added

- *(router)* server-sent events ([#218](https://github.com/tokio-rs/topcoat/pull/218))
- *(runtime)* add utility methods for signals with primitive types ([#214](https://github.com/tokio-rs/topcoat/pull/214))
- support wasm builds ([#191](https://github.com/tokio-rs/topcoat/pull/191))
- add support for routes that handle multiple (or all) methods ([#180](https://github.com/tokio-rs/topcoat/pull/180))

### Fixed

- browser hang when a text expression renders an owned string ([#201](https://github.com/tokio-rs/topcoat/pull/201))
- remove double type assertion in DOM binding ([#171](https://github.com/tokio-rs/topcoat/pull/171))
- return Bool surrogates from boolean event fields ([#168](https://github.com/tokio-rs/topcoat/pull/168))

### Other

- fix outdated documentation ([#211](https://github.com/tokio-rs/topcoat/pull/211))

## [0.3.1](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-v0.3.0...topcoat-runtime-v0.3.1) - 2026-07-20

### Fixed

- signal comment xss vulnerability ([#154](https://github.com/tokio-rs/topcoat/pull/154))

## [0.3.0](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-v0.2.0...topcoat-runtime-v0.3.0) - 2026-07-19

### Fixed

- handle procedure wire edge cases ([#139](https://github.com/tokio-rs/topcoat/pull/139))

### Other

- add runtime tests and build diff check ([#141](https://github.com/tokio-rs/topcoat/pull/141))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-v0.1.3...topcoat-runtime-v0.2.0) - 2026-07-19

### Fixed

- count browser string length in UTF-8 bytes ([#126](https://github.com/tokio-rs/topcoat/pull/126))

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.1.0](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-v0.0.4...topcoat-runtime-v0.1.0) - 2026-07-16

### Fixed

- [**breaking**] whitelist runtime browser bundle ([#107](https://github.com/tokio-rs/topcoat/pull/107))

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-runtime-v0.0.1) - 2026-07-14

### Other

- initial release
