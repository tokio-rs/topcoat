# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.3](https://github.com/tokio-rs/topcoat/compare/topcoat-view-v0.6.2...topcoat-view-v0.6.3) - 2026-08-22

### Other

- *(view)* remove `itoa` dependency in favor of `format_into` ([#366](https://github.com/tokio-rs/topcoat/pull/366))
- upgrade to rust 1.98 ([#367](https://github.com/tokio-rs/topcoat/pull/367))

## [0.6.2](https://github.com/tokio-rs/topcoat/compare/topcoat-view-v0.6.1...topcoat-view-v0.6.2) - 2026-08-18

### Fixed

- *(view)* make control-flow futures own their pattern bindings ([#360](https://github.com/tokio-rs/topcoat/pull/360))

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-view-v0.5.0...topcoat-view-v0.6.0) - 2026-08-17

### Added

- *(core)* stable component identity system ([#328](https://github.com/tokio-rs/topcoat/pull/328))
- *(core)* [**breaking**] make Cx detachable, remove CxBuilder ([#322](https://github.com/tokio-rs/topcoat/pull/322))
- *(view)* improve new arena rendering system ([#319](https://github.com/tokio-rs/topcoat/pull/319))
- *(view)* concurrent rendering ([#317](https://github.com/tokio-rs/topcoat/pull/317))
- implement NodeViewParts and AttributeValueViewParts for Cow<'static, str> ([#306](https://github.com/tokio-rs/topcoat/pull/306))
- use #[track_caller] where appropriate ([#262](https://github.com/tokio-rs/topcoat/pull/262))

### Fixed

- *(core)* keep macro bodies intact when formatting rust snippets ([#273](https://github.com/tokio-rs/topcoat/pull/273))
- *(view)* allow keyword element names ([#274](https://github.com/tokio-rs/topcoat/pull/274))

### Other

- *(view)* add promoted str optimization to avoid allocations ([#330](https://github.com/tokio-rs/topcoat/pull/330))
- *(view)* refactor new rendering system part 2 ([#320](https://github.com/tokio-rs/topcoat/pull/320))
- *(view)* add docs about concurrent rendering
- *(view)* add lowering step to high-level intermediate representation ([#316](https://github.com/tokio-rs/topcoat/pull/316))
- *(view)* consume view when rendering to improve performance ([#312](https://github.com/tokio-rs/topcoat/pull/312))
- sort imports
- make request and response dedicated modules

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-view-v0.4.0...topcoat-view-v0.5.0) - 2026-07-27

### Added

- [**breaking**] improve boolean attribute behavior and docs ([#179](https://github.com/tokio-rs/topcoat/pull/179))
- boxed components for cyclic component definition ([#176](https://github.com/tokio-rs/topcoat/pull/176))

### Other

- fix outdated documentation ([#211](https://github.com/tokio-rs/topcoat/pull/211))
- improve view macro docs

## [0.4.0](https://github.com/tokio-rs/topcoat/compare/topcoat-view-v0.3.1...topcoat-view-v0.4.0) - 2026-07-22

### Added

- add Alpine AJAX integration and example ([#158](https://github.com/tokio-rs/topcoat/pull/158))

### Fixed

- pretty printer forcing newline in empty html elements ([#161](https://github.com/tokio-rs/topcoat/pull/161))
- remove zmij, improve floating point formatting ([#156](https://github.com/tokio-rs/topcoat/pull/156))

## [0.3.1](https://github.com/tokio-rs/topcoat/compare/topcoat-view-v0.3.0...topcoat-view-v0.3.1) - 2026-07-20

### Fixed

- signal comment xss vulnerability ([#154](https://github.com/tokio-rs/topcoat/pull/154))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-view-v0.1.3...topcoat-view-v0.2.0) - 2026-07-19

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.1.2](https://github.com/tokio-rs/topcoat/compare/topcoat-view-v0.1.1...topcoat-view-v0.1.2) - 2026-07-17

### Added

- status code and headers in pages and layouts ([#120](https://github.com/tokio-rs/topcoat/pull/120))

## [0.0.4](https://github.com/tokio-rs/topcoat/compare/topcoat-view-v0.0.3...topcoat-view-v0.0.4) - 2026-07-15

### Other

- dtolnay itoa and zmij ([#102](https://github.com/tokio-rs/topcoat/pull/102))

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-view-v0.0.1) - 2026-07-14

### Other

- initial release
