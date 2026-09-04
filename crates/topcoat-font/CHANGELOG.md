# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.7.0](https://github.com/tokio-rs/topcoat/compare/topcoat-font-v0.6.2...topcoat-font-v0.7.0) - 2026-09-04

### Added

- [**breaking**] streaming SSR and `live!` + `emit!` regions ([#373](https://github.com/tokio-rs/topcoat/pull/373))

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-font-v0.5.0...topcoat-font-v0.6.0) - 2026-08-17

### Added

- *(view)* concurrent rendering ([#317](https://github.com/tokio-rs/topcoat/pull/317))
- use #[track_caller] where appropriate ([#262](https://github.com/tokio-rs/topcoat/pull/262))

### Fixed

- *(font)* support unicode ranges ending in E ([#307](https://github.com/tokio-rs/topcoat/pull/307))

### Other

- *(router)* [**breaking**] replace handler structs with traits in preparation for href ([#346](https://github.com/tokio-rs/topcoat/pull/346))
- *(core)* [**breaking**] turn fnv1a hash into a struct and add 128-bit variant ([#327](https://github.com/tokio-rs/topcoat/pull/327))
- sort imports
- make request and response dedicated modules

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-font-v0.4.0...topcoat-font-v0.5.0) - 2026-07-27

### Added

- support wasm builds ([#191](https://github.com/tokio-rs/topcoat/pull/191))
- add support for routes that handle multiple (or all) methods ([#180](https://github.com/tokio-rs/topcoat/pull/180))

### Fixed

- *(asset)* improve asset linking system ([#217](https://github.com/tokio-rs/topcoat/pull/217))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-font-v0.1.3...topcoat-font-v0.2.0) - 2026-07-19

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-font-v0.0.1) - 2026-07-14

### Other

- initial release
