# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0](https://github.com/tokio-rs/topcoat/compare/v0.7.0...v0.8.0) - 2026-09-05

### Added

- *(runtime)* [**breaking**] replace signal custom syntax with an ordinary rust function ([#384](https://github.com/tokio-rs/topcoat/pull/384))

### Fixed

- *(router)* treat an empty form or query value as None for Option<T> ([#383](https://github.com/tokio-rs/topcoat/pull/383))

## [0.7.0](https://github.com/tokio-rs/topcoat/compare/v0.6.2...v0.7.0) - 2026-09-04

### Added

- [**breaking**] streaming SSR and `live!` + `emit!` regions ([#373](https://github.com/tokio-rs/topcoat/pull/373))
- *(cli)* port retry on dev server init ([#364](https://github.com/tokio-rs/topcoat/pull/364))
- *(cli)* expose run method on `TopcoatCli` ([#363](https://github.com/tokio-rs/topcoat/pull/363))
- *(core)* add pretty printing impls for all `syn` types, remove `prettyplease` ([#372](https://github.com/tokio-rs/topcoat/pull/372))
- *(router)* implement `HrefTarget` for `&T: HrefTarget` to support dyn ([#370](https://github.com/tokio-rs/topcoat/pull/370))
- *(router)* is_current methods for Href, routes, and pages ([#362](https://github.com/tokio-rs/topcoat/pull/362))
- *(ui)* slightly adapt topcoat-ui default colors ([#379](https://github.com/tokio-rs/topcoat/pull/379))
- *(ui)* switch from feather icons to lucide in topcoat-ui ([#378](https://github.com/tokio-rs/topcoat/pull/378))

### Fixed

- *(cli)* don't reload over in-flight navigations on reconnect ([#365](https://github.com/tokio-rs/topcoat/pull/365))
- *(core)* memoize + borrowed async arg fails with AsyncFnOnce lifetime error ([#371](https://github.com/tokio-rs/topcoat/pull/371))
- *(runtime)* preserve boolean procedure results ([#375](https://github.com/tokio-rs/topcoat/pull/375))

### Other

- upgrade to rust 1.98 ([#367](https://github.com/tokio-rs/topcoat/pull/367))
- *(view)* remove `itoa` dependency in favor of `format_into` ([#366](https://github.com/tokio-rs/topcoat/pull/366))

## [0.6.2](https://github.com/tokio-rs/topcoat/compare/v0.6.1...v0.6.2) - 2026-08-18

### Added

- *(router)* add support for mounting a topcoat router as an axum service

### Fixed

- *(view)* make control-flow futures own their pattern bindings ([#360](https://github.com/tokio-rs/topcoat/pull/360))

## [0.6.1](https://github.com/tokio-rs/topcoat/compare/v0.6.0...v0.6.1) - 2026-08-18

### Fixed

- *(router)* rewrite through 'discover' routing ([#354](https://github.com/tokio-rs/topcoat/pull/354))
- *(runtime)* support await expressions in ExprBlock and ExprIf ([#352](https://github.com/tokio-rs/topcoat/pull/352))

## [0.6.0](https://github.com/tokio-rs/topcoat/compare/v0.5.0...v0.6.0) - 2026-08-17

### Added

- *(core)* [**breaking**] add scoped context using `cx.with(...)` ([#338](https://github.com/tokio-rs/topcoat/pull/338))
- *(router)* sitemaps ([#336](https://github.com/tokio-rs/topcoat/pull/336))
- *(core)* seal Cx on detach
- *(core)* [**breaking**] make Cx detachable, remove CxBuilder ([#322](https://github.com/tokio-rs/topcoat/pull/322))
- *(core)* [**breaking**] specify as_ref manually on memoized functions ([#310](https://github.com/tokio-rs/topcoat/pull/310))
- *(router)* [**breaking**] global origin policy ([#276](https://github.com/tokio-rs/topcoat/pull/276))
- *(router)* [**breaking**] replace path parameter attribute macro ([#242](https://github.com/tokio-rs/topcoat/pull/242))
- use #[track_caller] where appropriate ([#262](https://github.com/tokio-rs/topcoat/pull/262))
- *(view)* concurrent rendering ([#317](https://github.com/tokio-rs/topcoat/pull/317))
- *(cli)* warn on version mismatch between topcoat and cli ([#305](https://github.com/tokio-rs/topcoat/pull/305))
- *(cookie)* protect cookie jar from being written to after response… ([#324](https://github.com/tokio-rs/topcoat/pull/324))
- *(core)* use 128-bit hash instead of Clone and Eq for memoization ([#337](https://github.com/tokio-rs/topcoat/pull/337))
- *(view)* improve new arena rendering system ([#319](https://github.com/tokio-rs/topcoat/pull/319))
- *(router)* [**breaking**] add unused layer sanity check ([#300](https://github.com/tokio-rs/topcoat/pull/300))
- *(router)* use `impl AsRef<str>` for route error urls ([#351](https://github.com/tokio-rs/topcoat/pull/351))
- *(router)* `href!` macro ([#350](https://github.com/tokio-rs/topcoat/pull/350))
- *(router)* request rewriting ([#347](https://github.com/tokio-rs/topcoat/pull/347))
- *(router)* add matched endpoint path to request context ([#318](https://github.com/tokio-rs/topcoat/pull/318))
- *(router)* [**breaking**] add not_found macro and no longer run layers and layouts by default on unmatched requests ([#298](https://github.com/tokio-rs/topcoat/pull/298))
- *(router)* add a too-many-requests error with a Retry-After hint ([#267](https://github.com/tokio-rs/topcoat/pull/267))
- *(router)* add Js and Wasm response wrappers ([#268](https://github.com/tokio-rs/topcoat/pull/268))
- *(router)* add a service-unavailable error with a Retry-After hint ([#266](https://github.com/tokio-rs/topcoat/pull/266))
- *(router)* [**breaking**] request body limits ([#233](https://github.com/tokio-rs/topcoat/pull/233))
- *(ui)* add 17 new topcoat-ui components ([#341](https://github.com/tokio-rs/topcoat/pull/341))
- *(core)* stable component identity system ([#328](https://github.com/tokio-rs/topcoat/pull/328))
- implement NodeViewParts and AttributeValueViewParts for Cow<'static, str> ([#306](https://github.com/tokio-rs/topcoat/pull/306))

### Fixed

- fix docs issues
- fix doc tests with default features
- *(asset)* [**breaking**] write the bundle next to the executable it was scanned from ([#243](https://github.com/tokio-rs/topcoat/pull/243))
- *(asset)* ignore false-positive asset scans with empty paths ([#280](https://github.com/tokio-rs/topcoat/pull/280))
- *(asset)* register one route per bundled file ([#249](https://github.com/tokio-rs/topcoat/pull/249))
- *(cli)* add '/ws' to exempt OriginPolicy on dev ([#348](https://github.com/tokio-rs/topcoat/pull/348))
- *(cli)* exclude build script outputs from final output detection ([#301](https://github.com/tokio-rs/topcoat/pull/301))
- *(core)* detect recursive memoized calls ([#278](https://github.com/tokio-rs/topcoat/pull/278))
- *(core)* keep macro bodies intact when formatting rust snippets ([#273](https://github.com/tokio-rs/topcoat/pull/273))
- *(datastar)* reject multiline selectors ([#256](https://github.com/tokio-rs/topcoat/pull/256))
- *(font)* support unicode ranges ending in E ([#307](https://github.com/tokio-rs/topcoat/pull/307))
- *(router)* isolate request panics ([#257](https://github.com/tokio-rs/topcoat/pull/257))
- *(router)* reject invalid route signatures at parse time ([#241](https://github.com/tokio-rs/topcoat/pull/241))
- include docs and visibility in routes, procedures, layers ([#232](https://github.com/tokio-rs/topcoat/pull/232))
- *(runtime)* support signals outside the body ([#334](https://github.com/tokio-rs/topcoat/pull/334))
- *(runtime)* let push_str accept an owned string surrogate ([#269](https://github.com/tokio-rs/topcoat/pull/269))
- *(runtime)* reject non-2xx shard responses instead of rendering them ([#247](https://github.com/tokio-rs/topcoat/pull/247))
- *(runtime)* render f64 text the way Rust's Display does ([#245](https://github.com/tokio-rs/topcoat/pull/245))
- *(runtime)* match Rust semantics for string comparison and trim ([#244](https://github.com/tokio-rs/topcoat/pull/244))
- *(runtime)* reject invalid procedure signatures at parse time ([#230](https://github.com/tokio-rs/topcoat/pull/230))
- *(view)* allow keyword element names ([#274](https://github.com/tokio-rs/topcoat/pull/274))

### Other

- *(router)* [**breaking**] replace handler structs with traits in preparation for href ([#346](https://github.com/tokio-rs/topcoat/pull/346))
- sort imports
- make request and response dedicated modules
- decrease logo size
- add readme logo
- *(core)* [**breaking**] turn fnv1a hash into a struct and add 128-bit variant ([#327](https://github.com/tokio-rs/topcoat/pull/327))
- *(view)* add promoted str optimization to avoid allocations ([#330](https://github.com/tokio-rs/topcoat/pull/330))
- fix memoize docs stale example
- *(view)* consume view when rendering to improve performance ([#312](https://github.com/tokio-rs/topcoat/pull/312))
- *(router)* rename erased constant for more readable profiler and debugger traces ([#329](https://github.com/tokio-rs/topcoat/pull/329))
- *(router)* remove PathBuf Arc pointer indirection ([#323](https://github.com/tokio-rs/topcoat/pull/323))
- merge router service and serve into single file
- [**breaking**] return ContentTooLargeError instead of LengthLimitError from to_bytes ([#263](https://github.com/tokio-rs/topcoat/pull/263))
- *(runtime)* note that page guards do not cover shard endpoints ([#251](https://github.com/tokio-rs/topcoat/pull/251))
- *(view)* refactor new rendering system part 2 ([#320](https://github.com/tokio-rs/topcoat/pull/320))
- *(view)* add docs about concurrent rendering
- *(view)* add lowering step to high-level intermediate representation ([#316](https://github.com/tokio-rs/topcoat/pull/316))

## [0.5.0](https://github.com/tokio-rs/topcoat/compare/v0.4.0...v0.5.0) - 2026-07-27

### Added

- *(datastar)* add datastar support ([#219](https://github.com/tokio-rs/topcoat/pull/219))
- *(router)* server-sent events ([#218](https://github.com/tokio-rs/topcoat/pull/218))
- *(mail)* email prototype ([#216](https://github.com/tokio-rs/topcoat/pull/216))
- *(runtime)* add utility methods for signals with primitive types ([#214](https://github.com/tokio-rs/topcoat/pull/214))
- websocket support ([#195](https://github.com/tokio-rs/topcoat/pull/195))
- support wasm builds ([#191](https://github.com/tokio-rs/topcoat/pull/191))
- support unix listeners ([#190](https://github.com/tokio-rs/topcoat/pull/190))
- support mounting tower services as routes in the router ([#184](https://github.com/tokio-rs/topcoat/pull/184))
- make page HTTP methods customizable ([#181](https://github.com/tokio-rs/topcoat/pull/181))
- add support for routes that handle multiple (or all) methods ([#180](https://github.com/tokio-rs/topcoat/pull/180))
- topcoat dev reports more build errors ([#208](https://github.com/tokio-rs/topcoat/pull/208))
- support wasm asset bundling ([#199](https://github.com/tokio-rs/topcoat/pull/199))
- *(router)* validate websocket handshake keys ([#215](https://github.com/tokio-rs/topcoat/pull/215))
- [**breaking**] improve boolean attribute behavior and docs ([#179](https://github.com/tokio-rs/topcoat/pull/179))
- boxed components for cyclic component definition ([#176](https://github.com/tokio-rs/topcoat/pull/176))

### Fixed

- *(asset)* improve asset linking system ([#217](https://github.com/tokio-rs/topcoat/pull/217))
- formatter exiting with code 0 on failed stdin formatting ([#207](https://github.com/tokio-rs/topcoat/pull/207))
- keep embedded asset declarations alive on MSVC builds ([#170](https://github.com/tokio-rs/topcoat/pull/170))
- topcoat dev hot reload never succeeds on Windows (exe file lock) ([#169](https://github.com/tokio-rs/topcoat/pull/169))
- browser hang when a text expression renders an owned string ([#201](https://github.com/tokio-rs/topcoat/pull/201))
- remove double type assertion in DOM binding ([#171](https://github.com/tokio-rs/topcoat/pull/171))
- return Bool surrogates from boolean event fields ([#168](https://github.com/tokio-rs/topcoat/pull/168))
- topcoat formatter breaking at signal syntax ([#172](https://github.com/tokio-rs/topcoat/pull/172))

### Other

- improve readme
- *(mail)* mail example and guide ([#221](https://github.com/tokio-rs/topcoat/pull/221))
- add new roadmap items
- clarify module router registration
- fix outdated documentation ([#211](https://github.com/tokio-rs/topcoat/pull/211))
- [**breaking**] change order of AssetConfig::hosted_at
- clarify module router paths and parameter handling ([#185](https://github.com/tokio-rs/topcoat/pull/185))
- [**breaking**] dedicated router error module ([#183](https://github.com/tokio-rs/topcoat/pull/183))
- improve router macro docs ([#178](https://github.com/tokio-rs/topcoat/pull/178))
- add sitemaps to roadmap
- [**breaking**] pass layouts the rendered Result<View> instead of a Slot future ([#166](https://github.com/tokio-rs/topcoat/pull/166))
- improve router macro docs
- improve view macro docs

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
