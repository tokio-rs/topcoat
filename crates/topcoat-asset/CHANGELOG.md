# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-asset-v0.5.0...topcoat-asset-v0.6.0) - 2026-08-17

### Added

- *(view)* concurrent rendering ([#317](https://github.com/tokio-rs/topcoat/pull/317))
- use #[track_caller] where appropriate ([#262](https://github.com/tokio-rs/topcoat/pull/262))

### Fixed

- *(asset)* ignore false-positive asset scans with empty paths ([#280](https://github.com/tokio-rs/topcoat/pull/280))
- *(asset)* [**breaking**] write the bundle next to the executable it was scanned from ([#243](https://github.com/tokio-rs/topcoat/pull/243))
- *(asset)* register one route per bundled file ([#249](https://github.com/tokio-rs/topcoat/pull/249))

### Other

- *(router)* [**breaking**] replace handler structs with traits in preparation for href ([#346](https://github.com/tokio-rs/topcoat/pull/346))
- *(core)* [**breaking**] turn fnv1a hash into a struct and add 128-bit variant ([#327](https://github.com/tokio-rs/topcoat/pull/327))
- sort imports
- make request and response dedicated modules

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-asset-v0.4.0...topcoat-asset-v0.5.0) - 2026-07-27

### Added

- support wasm builds ([#191](https://github.com/tokio-rs/topcoat/pull/191))
- add support for routes that handle multiple (or all) methods ([#180](https://github.com/tokio-rs/topcoat/pull/180))

### Fixed

- *(asset)* improve asset linking system ([#217](https://github.com/tokio-rs/topcoat/pull/217))

### Other

- fix outdated documentation ([#211](https://github.com/tokio-rs/topcoat/pull/211))
- [**breaking**] change order of AssetConfig::hosted_at

## [0.4.0](https://github.com/tokio-rs/topcoat/compare/topcoat-asset-v0.3.1...topcoat-asset-v0.4.0) - 2026-07-22

### Added

- bundler prallelism ([#157](https://github.com/tokio-rs/topcoat/pull/157))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-asset-v0.1.3...topcoat-asset-v0.2.0) - 2026-07-19

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-asset-v0.0.1) - 2026-07-14

### Other

- initial release
