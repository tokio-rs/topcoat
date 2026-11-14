# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.12.2](https://github.com/tokio-rs/topcoat/compare/topcoat-l10n-v0.12.1...topcoat-l10n-v0.12.2) - 2026-11-14

### Other

- rewrite benchmarks with new frameworks and visualization ([#613](https://github.com/tokio-rs/topcoat/pull/613))

## [0.12.0](https://github.com/tokio-rs/topcoat/compare/topcoat-l10n-v0.11.0...topcoat-l10n-v0.12.0) - 2026-11-09

### Added

- [**breaking**] require leading `cx: &Cx` and capture context in view macros ([#581](https://github.com/tokio-rs/topcoat/pull/581))
- [**breaking**] lenient macros ([#580](https://github.com/tokio-rs/topcoat/pull/580))
- [**breaking**] add formatting with `topcoat fmt` for all topcoat macros ([#579](https://github.com/tokio-rs/topcoat/pull/579))
- [**breaking**] unify macro syntax ([#571](https://github.com/tokio-rs/topcoat/pull/571))
- *(router)* route context with `#[route(with(...))]` ([#566](https://github.com/tokio-rs/topcoat/pull/566))
- *(core)* [**breaking**] unify `app_context` and `request_context` into one `context` ([#562](https://github.com/tokio-rs/topcoat/pull/562))

## [0.11.0](https://github.com/tokio-rs/topcoat/compare/topcoat-l10n-v0.0.0...topcoat-l10n-v0.11.0) - 2026-10-24

### Added

- *(l10n)* localization improvements, date and time support, number formatting feature flag ([#559](https://github.com/tokio-rs/topcoat/pull/559))
- *(demo)* admin demo localizations ([#557](https://github.com/tokio-rs/topcoat/pull/557))
- *(l10n)* localization support with fluent ([#556](https://github.com/tokio-rs/topcoat/pull/556))
