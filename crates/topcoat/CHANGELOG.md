# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-v0.4.0...topcoat-v0.5.0) - 2026-07-23

### Added

- support mounting tower services as routes in the router ([#184](https://github.com/tokio-rs/topcoat/pull/184))
- make page HTTP methods customizable ([#181](https://github.com/tokio-rs/topcoat/pull/181))
- add support for routes that handle multiple (or all) methods ([#180](https://github.com/tokio-rs/topcoat/pull/180))

### Other

- [**breaking**] dedicated router error module ([#183](https://github.com/tokio-rs/topcoat/pull/183))
- improve router macro docs ([#178](https://github.com/tokio-rs/topcoat/pull/178))
- add sitemaps to roadmap
- [**breaking**] pass layouts the rendered Result<View> instead of a Slot future ([#166](https://github.com/tokio-rs/topcoat/pull/166))

## [0.4.0](https://github.com/tokio-rs/topcoat/compare/topcoat-v0.3.1...topcoat-v0.4.0) - 2026-07-22

### Added

- add Alpine AJAX integration and example ([#158](https://github.com/tokio-rs/topcoat/pull/158))

### Other

- fix hyphenation in 'fullstack' to 'full-stack' ([#163](https://github.com/tokio-rs/topcoat/pull/163))
- add WebTransport to roadmap ([#160](https://github.com/tokio-rs/topcoat/pull/160))

## [0.3.1](https://github.com/tokio-rs/topcoat/compare/topcoat-v0.3.0...topcoat-v0.3.1) - 2026-07-20

### Other

- add note on build time improvements

## [0.3.0](https://github.com/tokio-rs/topcoat/compare/topcoat-v0.2.0...topcoat-v0.3.0) - 2026-07-19

### Added

- manual reload in dev server ([#144](https://github.com/tokio-rs/topcoat/pull/144))

### Other

- improve readme
- fix clippy
- add roadmap
- improve topcoat ui readme

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-v0.1.3...topcoat-v0.2.0) - 2026-07-19

### Added

- [**breaking**] compression ([#133](https://github.com/tokio-rs/topcoat/pull/133))
- [**breaking**] graceful shutdown ([#132](https://github.com/tokio-rs/topcoat/pull/132))

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))
- add documentation for topcoat-ui ([#130](https://github.com/tokio-rs/topcoat/pull/130))

## [0.1.3](https://github.com/tokio-rs/topcoat/compare/topcoat-v0.1.2...topcoat-v0.1.3) - 2026-07-17

### Other

- add reactivity guide ([#123](https://github.com/tokio-rs/topcoat/pull/123))
- fullstack -> full-stack ([#122](https://github.com/tokio-rs/topcoat/pull/122))
- readme client reactivity section
- readme client reactivity section
- client reactivity in readme
- readme client reactivity section
- improve view macro example

## [0.1.2](https://github.com/tokio-rs/topcoat/compare/topcoat-v0.1.1...topcoat-v0.1.2) - 2026-07-17

### Added

- status code and headers in pages and layouts ([#120](https://github.com/tokio-rs/topcoat/pull/120))
- add optional context accessors ([#115](https://github.com/tokio-rs/topcoat/pull/115))

## [0.1.1](https://github.com/tokio-rs/topcoat/compare/topcoat-v0.1.0...topcoat-v0.1.1) - 2026-07-16

### Added

- sessions ([#109](https://github.com/tokio-rs/topcoat/pull/109))

## [0.0.4](https://github.com/tokio-rs/topcoat/compare/topcoat-v0.0.3...topcoat-v0.0.4) - 2026-07-15

### Added

- tower layers ([#103](https://github.com/tokio-rs/topcoat/pull/103))

## [0.0.3](https://github.com/tokio-rs/topcoat/compare/topcoat-v0.0.2...topcoat-v0.0.3) - 2026-07-14

### Other

- link docs.rs instead of github ([#100](https://github.com/tokio-rs/topcoat/pull/100))

## [0.0.1](https://github.com/tokio-rs/topcoat/compare/topcoat-v0.0.0...topcoat-v0.0.1) - 2026-07-14

### Other

- initial release
