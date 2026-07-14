# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1](https://github.com/tokio-rs/topcoat/releases/tag/topcoat-runtime-v0.0.1) - 2026-07-14

### Other

- add udeps workflow ([#96](https://github.com/tokio-rs/topcoat/pull/96))
- Work around release-plz bug
- Use dotted topcoat paths
- Release plz ([#92](https://github.com/tokio-rs/topcoat/pull/92))
- Proc macro crate ([#90](https://github.com/tokio-rs/topcoat/pull/90))
- Share version and edition across workspace
- Refactor ast/runtime split crates into grammar crates ([#83](https://github.com/tokio-rs/topcoat/pull/83))
- Refactor HTML escaping and view part design ([#80](https://github.com/tokio-rs/topcoat/pull/80))
- Add cx parameter to *ViewParts traits ([#73](https://github.com/tokio-rs/topcoat/pull/73))
- Optimize 'static str unescaped paths ([#72](https://github.com/tokio-rs/topcoat/pull/72))
- Purge special characters ([#61](https://github.com/tokio-rs/topcoat/pull/61))
- Remove README headlines
- Add font system ([#48](https://github.com/tokio-rs/topcoat/pull/48))
- Add HTMX support ([#47](https://github.com/tokio-rs/topcoat/pull/47))
- Bring back shards ([#46](https://github.com/tokio-rs/topcoat/pull/46))
- Enable pedantic lints ([#45](https://github.com/tokio-rs/topcoat/pull/45))
- Ditch mod.rs ([#42](https://github.com/tokio-rs/topcoat/pull/42))
- Improve build times ([#41](https://github.com/tokio-rs/topcoat/pull/41))
- Better docs ([#37](https://github.com/tokio-rs/topcoat/pull/37))
- The mega refactor ([#34](https://github.com/tokio-rs/topcoat/pull/34))
- Router refactor ([#19](https://github.com/tokio-rs/topcoat/pull/19))
- Better (de-)hydration ([#18](https://github.com/tokio-rs/topcoat/pull/18))
- Rename Action to Procedure ([#17](https://github.com/tokio-rs/topcoat/pull/17))
- Add Future<T> surrogate for JS ([#16](https://github.com/tokio-rs/topcoat/pull/16))
- Actions ([#15](https://github.com/tokio-rs/topcoat/pull/15))
- Loops ([#14](https://github.com/tokio-rs/topcoat/pull/14))
- More surrogates ([#13](https://github.com/tokio-rs/topcoat/pull/13))
- Switch to self-describing runtime data format ([#12](https://github.com/tokio-rs/topcoat/pull/12))
- Merge develop ([#11](https://github.com/tokio-rs/topcoat/pull/11))
- New example project and docs ([#9](https://github.com/tokio-rs/topcoat/pull/9))
- Add Signal::get
- Fix dehydration
- Add Event surrogate
- Surrogate progress
- Change signals to be references again
- Add string surrogates
- Add dedicated type erased top level topcoat error type
- Rename island to shard
- Refactor surrogates
- Fix closures
- Add double surrogate trait
- Add ToJs
- Switch to RefCast derive
- Add new surrogate impls
- optimize ViewPart collection with ViewParts buffer
- Add new ViewParts trait split
- WIP
- Anonymize locals and externals
- Add runtime support for f64
- Optimize JS string building
- Add support for more expressions
- Add new macro-time JS cross-compiler
- Do it again
- WIP
- WIP
- Add interop crate
- More str improvements
- Refactor imports
- Add support for string literals
- WIP
- Improve generated comments
- Fix warnings
- Remove unnecessary param tracking
- Add type inference for closures
- Add very nice code
- Add new Value system
- WIP
- Switch to JS function based evaluation
- WIP
- WIP
- WIP
- WIP
- Implement one way binding for signals with expr macro
- WIP
- Add attribute kind
- Start new JS runtime
- Add runtime crate
