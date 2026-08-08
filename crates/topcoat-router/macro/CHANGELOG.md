# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-macro-v0.5.0...topcoat-router-macro-v0.6.0) - 2026-08-08

### Added

- *(core)* [**breaking**] make Cx detachable, remove CxBuilder ([#322](https://github.com/tokio-rs/topcoat/pull/322))
- *(router)* [**breaking**] add not_found macro and no longer run layers and layouts by default on unmatched requests ([#298](https://github.com/tokio-rs/topcoat/pull/298))
- *(router)* [**breaking**] replace path parameter attribute macro ([#242](https://github.com/tokio-rs/topcoat/pull/242))

### Other

- *(router)* remove PathBuf Arc pointer indirection ([#323](https://github.com/tokio-rs/topcoat/pull/323))
- make request and response dedicated modules

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-macro-v0.4.0...topcoat-router-macro-v0.5.0) - 2026-07-27

### Added

- *(router)* server-sent events ([#218](https://github.com/tokio-rs/topcoat/pull/218))
- make page HTTP methods customizable ([#181](https://github.com/tokio-rs/topcoat/pull/181))
- add support for routes that handle multiple (or all) methods ([#180](https://github.com/tokio-rs/topcoat/pull/180))

### Other

- fix outdated documentation ([#211](https://github.com/tokio-rs/topcoat/pull/211))
- clarify module router paths and parameter handling ([#185](https://github.com/tokio-rs/topcoat/pull/185))
- [**breaking**] dedicated router error module ([#183](https://github.com/tokio-rs/topcoat/pull/183))
- improve router macro docs ([#178](https://github.com/tokio-rs/topcoat/pull/178))
- improve router macro docs
- [**breaking**] pass layouts the rendered Result<View> instead of a Slot future ([#166](https://github.com/tokio-rs/topcoat/pull/166))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-macro-v0.1.3...topcoat-router-macro-v0.2.0) - 2026-07-19

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-router-macro-v0.0.1) - 2026-07-14

### Other

- initial release
