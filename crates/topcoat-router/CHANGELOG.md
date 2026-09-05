# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-v0.7.0...topcoat-router-v0.8.0) - 2026-09-05

### Fixed

- *(router)* treat an empty form or query value as None for Option<T> ([#383](https://github.com/tokio-rs/topcoat/pull/383))

## [0.7.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-v0.6.2...topcoat-router-v0.7.0) - 2026-09-04

### Added

- [**breaking**] streaming SSR and `live!` + `emit!` regions ([#373](https://github.com/tokio-rs/topcoat/pull/373))
- *(router)* implement `HrefTarget` for `&T: HrefTarget` to support dyn ([#370](https://github.com/tokio-rs/topcoat/pull/370))
- *(router)* is_current methods for Href, routes, and pages ([#362](https://github.com/tokio-rs/topcoat/pull/362))

### Other

- upgrade to rust 1.98 ([#367](https://github.com/tokio-rs/topcoat/pull/367))

## [0.6.2](https://github.com/tokio-rs/topcoat/compare/topcoat-router-v0.6.1...topcoat-router-v0.6.2) - 2026-08-18

### Added

- *(router)* add support for mounting a topcoat router as an axum service

## [0.6.1](https://github.com/tokio-rs/topcoat/compare/topcoat-router-v0.6.0...topcoat-router-v0.6.1) - 2026-08-18

### Fixed

- *(router)* rewrite through 'discover' routing ([#354](https://github.com/tokio-rs/topcoat/pull/354))

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-v0.5.0...topcoat-router-v0.6.0) - 2026-08-17

### Added

- *(router)* use `impl AsRef<str>` for route error urls ([#351](https://github.com/tokio-rs/topcoat/pull/351))
- *(router)* `href!` macro ([#350](https://github.com/tokio-rs/topcoat/pull/350))
- *(router)* request rewriting ([#347](https://github.com/tokio-rs/topcoat/pull/347))
- *(core)* [**breaking**] add scoped context using `cx.with(...)` ([#338](https://github.com/tokio-rs/topcoat/pull/338))
- *(router)* sitemaps ([#336](https://github.com/tokio-rs/topcoat/pull/336))
- *(core)* [**breaking**] make Cx detachable, remove CxBuilder ([#322](https://github.com/tokio-rs/topcoat/pull/322))
- *(view)* improve new arena rendering system ([#319](https://github.com/tokio-rs/topcoat/pull/319))
- *(router)* add matched endpoint path to request context ([#318](https://github.com/tokio-rs/topcoat/pull/318))
- *(view)* concurrent rendering ([#317](https://github.com/tokio-rs/topcoat/pull/317))
- *(core)* [**breaking**] specify as_ref manually on memoized functions ([#310](https://github.com/tokio-rs/topcoat/pull/310))
- *(router)* [**breaking**] add unused layer sanity check ([#300](https://github.com/tokio-rs/topcoat/pull/300))
- *(router)* [**breaking**] add not_found macro and no longer run layers and layouts by default on unmatched requests ([#298](https://github.com/tokio-rs/topcoat/pull/298))
- *(router)* [**breaking**] global origin policy ([#276](https://github.com/tokio-rs/topcoat/pull/276))
- *(router)* add a too-many-requests error with a Retry-After hint ([#267](https://github.com/tokio-rs/topcoat/pull/267))
- *(router)* add Js and Wasm response wrappers ([#268](https://github.com/tokio-rs/topcoat/pull/268))
- *(router)* add a service-unavailable error with a Retry-After hint ([#266](https://github.com/tokio-rs/topcoat/pull/266))
- use #[track_caller] where appropriate ([#262](https://github.com/tokio-rs/topcoat/pull/262))
- *(router)* [**breaking**] replace path parameter attribute macro ([#242](https://github.com/tokio-rs/topcoat/pull/242))
- *(router)* [**breaking**] request body limits ([#233](https://github.com/tokio-rs/topcoat/pull/233))

### Fixed

- fix docs issues
- *(router)* isolate request panics ([#257](https://github.com/tokio-rs/topcoat/pull/257))
- *(router)* reject invalid route signatures at parse time ([#241](https://github.com/tokio-rs/topcoat/pull/241))

### Other

- *(router)* [**breaking**] replace handler structs with traits in preparation for href ([#346](https://github.com/tokio-rs/topcoat/pull/346))
- *(router)* rename erased constant for more readable profiler and debugger traces ([#329](https://github.com/tokio-rs/topcoat/pull/329))
- *(router)* remove PathBuf Arc pointer indirection ([#323](https://github.com/tokio-rs/topcoat/pull/323))
- sort imports
- make request and response dedicated modules
- merge router service and serve into single file
- [**breaking**] return ContentTooLargeError instead of LengthLimitError from to_bytes ([#263](https://github.com/tokio-rs/topcoat/pull/263))

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-v0.4.0...topcoat-router-v0.5.0) - 2026-07-27

### Added

- *(router)* server-sent events ([#218](https://github.com/tokio-rs/topcoat/pull/218))
- *(mail)* email prototype ([#216](https://github.com/tokio-rs/topcoat/pull/216))
- *(router)* validate websocket handshake keys ([#215](https://github.com/tokio-rs/topcoat/pull/215))
- websocket support ([#195](https://github.com/tokio-rs/topcoat/pull/195))
- support wasm builds ([#191](https://github.com/tokio-rs/topcoat/pull/191))
- support unix listeners ([#190](https://github.com/tokio-rs/topcoat/pull/190))
- support mounting tower services as routes in the router ([#184](https://github.com/tokio-rs/topcoat/pull/184))
- make page HTTP methods customizable ([#181](https://github.com/tokio-rs/topcoat/pull/181))
- add support for routes that handle multiple (or all) methods ([#180](https://github.com/tokio-rs/topcoat/pull/180))

### Other

- clarify module router registration
- fix outdated documentation ([#211](https://github.com/tokio-rs/topcoat/pull/211))
- clarify module router paths and parameter handling ([#185](https://github.com/tokio-rs/topcoat/pull/185))
- [**breaking**] dedicated router error module ([#183](https://github.com/tokio-rs/topcoat/pull/183))
- improve router macro docs ([#178](https://github.com/tokio-rs/topcoat/pull/178))
- improve router macro docs
- [**breaking**] pass layouts the rendered Result<View> instead of a Slot future ([#166](https://github.com/tokio-rs/topcoat/pull/166))

## [0.2.0](https://github.com/tokio-rs/topcoat/compare/topcoat-router-v0.1.3...topcoat-router-v0.2.0) - 2026-07-19

### Added

- [**breaking**] compression ([#133](https://github.com/tokio-rs/topcoat/pull/133))
- [**breaking**] graceful shutdown ([#132](https://github.com/tokio-rs/topcoat/pull/132))

### Other

- add annotations to show feature flags in docs.rs ([#127](https://github.com/tokio-rs/topcoat/pull/127))

## [0.1.2](https://github.com/tokio-rs/topcoat/compare/topcoat-router-v0.1.1...topcoat-router-v0.1.2) - 2026-07-17

### Added

- status code and headers in pages and layouts ([#120](https://github.com/tokio-rs/topcoat/pull/120))

## [0.0.4](https://github.com/tokio-rs/topcoat/compare/topcoat-router-v0.0.3...topcoat-router-v0.0.4) - 2026-07-15

### Added

- tower layers ([#103](https://github.com/tokio-rs/topcoat/pull/103))

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-router-v0.0.1) - 2026-07-14

### Other

- initial release
