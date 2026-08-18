# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.1](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-grammar-v0.6.0...topcoat-runtime-grammar-v0.6.1) - 2026-08-18

### Fixed

- *(runtime)* support await expressions in ExprBlock and ExprIf ([#352](https://github.com/tokio-rs/topcoat/pull/352))

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-grammar-v0.5.0...topcoat-runtime-grammar-v0.6.0) - 2026-08-17

### Added

- *(view)* improve new arena rendering system ([#319](https://github.com/tokio-rs/topcoat/pull/319))
- *(view)* concurrent rendering ([#317](https://github.com/tokio-rs/topcoat/pull/317))

### Fixed

- include docs and visibility in routes, procedures, layers ([#232](https://github.com/tokio-rs/topcoat/pull/232))
- *(runtime)* reject invalid procedure signatures at parse time ([#230](https://github.com/tokio-rs/topcoat/pull/230))

### Other

- *(router)* [**breaking**] replace handler structs with traits in preparation for href ([#346](https://github.com/tokio-rs/topcoat/pull/346))
- *(router)* rename erased constant for more readable profiler and debugger traces ([#329](https://github.com/tokio-rs/topcoat/pull/329))
- sort imports
- make request and response dedicated modules

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-grammar-v0.4.0...topcoat-runtime-grammar-v0.5.0) - 2026-07-27

### Added

- *(router)* server-sent events ([#218](https://github.com/tokio-rs/topcoat/pull/218))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-grammar-v0.1.3...topcoat-runtime-grammar-v0.2.0) - 2026-07-19

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.1.3](https://github.com/tokio-rs/topcoat/compare/topcoat-runtime-grammar-v0.1.2...topcoat-runtime-grammar-v0.1.3) - 2026-07-17

### Fixed

- shards no longer need cx =>

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-runtime-grammar-v0.0.1) - 2026-07-14

### Other

- initial release
