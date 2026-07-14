# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-core-v0.0.1) - 2026-07-14

### Other

- add udeps workflow ([#96](https://github.com/tokio-rs/topcoat/pull/96))
- Merge crates ([#94](https://github.com/tokio-rs/topcoat/pull/94))
- Work around release-plz bug
- Use dotted topcoat paths
- Release plz ([#92](https://github.com/tokio-rs/topcoat/pull/92))
- Proc macro crate ([#90](https://github.com/tokio-rs/topcoat/pull/90))
- Share version and edition across workspace
- Refactor ast/runtime split crates into grammar crates ([#83](https://github.com/tokio-rs/topcoat/pull/83))
- Icon improvements ([#76](https://github.com/tokio-rs/topcoat/pull/76))
- Add CxBuilder and CxTestBuilder ([#75](https://github.com/tokio-rs/topcoat/pull/75))
- Purge special characters ([#61](https://github.com/tokio-rs/topcoat/pull/61))
- Remove README headlines
- Add font system ([#48](https://github.com/tokio-rs/topcoat/pull/48))
- Enable pedantic lints ([#45](https://github.com/tokio-rs/topcoat/pull/45))
- Fix intradoc links ([#43](https://github.com/tokio-rs/topcoat/pull/43))
- Ditch mod.rs ([#42](https://github.com/tokio-rs/topcoat/pull/42))
- Improve build times ([#41](https://github.com/tokio-rs/topcoat/pull/41))
- Remove unused deps ([#39](https://github.com/tokio-rs/topcoat/pull/39))
- Move tests to their respective crates ([#38](https://github.com/tokio-rs/topcoat/pull/38))
- Better docs ([#37](https://github.com/tokio-rs/topcoat/pull/37))
- Refactor ([#33](https://github.com/tokio-rs/topcoat/pull/33))
- Add abort docs and tests
- Improve path naming conventions
- Cookies ([#24](https://github.com/tokio-rs/topcoat/pull/24))
- Actions ([#15](https://github.com/tokio-rs/topcoat/pull/15))
- Merge develop ([#11](https://github.com/tokio-rs/topcoat/pull/11))
- New example project and docs ([#9](https://github.com/tokio-rs/topcoat/pull/9))
- Add dedicated type erased top level topcoat error type
- Add cache control headers for assets
- Add more attribute value variants
- Improve type error printing
- Get rid of task local for cx
- Pass cx to pages and layouts directly instead of through task local
- Add Cx::default
- Use anymap for memo cache
- Add proper Cx constructor
- Switch to anymap for state
- Move router specific request state to router crate
- Add request state
- Add app state tests
- More docs
- Return option from AppState
- Add expect instead of unwrap
- Add app_state docs
- Add docs for router app state registration
- Add app state
- Extract pretty printing into separate crate
- Add feature check and fix feature issues
- Add abort demo
- Remove unnecessary box
- Rewrite MemoizeCache to return direct references
- Revert "Towel"
- Towel
- Move raw_path_params hook to router
- Add param parsing
- Add memoize tests
- Fix example project
- Add memoize macro docs
- Add comments
- Add doc hidden
- Rename params
- Extract shared code
- Add OnceLock for sync path
- Prevent HashDoS
- Add async version
- We did it bros
- WIP
- Add MemoizeKey impl
- Remove type id paramter
- Refactor memo cache
- Add concrete map approach
- WIP 2
- WIP
- Add CxId
- First working memoize example
- Memoization progress
- Memoize method
- WIP
- WIP
- Move context to core crate
- Fix CI failures
- Add new formatter design
