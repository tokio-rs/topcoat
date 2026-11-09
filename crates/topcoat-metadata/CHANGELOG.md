# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.12.0](https://github.com/tokio-rs/topcoat/compare/topcoat-metadata-v0.0.0...topcoat-metadata-v0.12.0) - 2026-11-09

### Added

- *(router)* [**breaking**] add named path parameter accessors and href arguments ([#594](https://github.com/tokio-rs/topcoat/pull/594))
- [**breaking**] simpler metadata builders ([#586](https://github.com/tokio-rs/topcoat/pull/586))
- [**breaking**] require leading `cx: &Cx` and capture context in view macros ([#581](https://github.com/tokio-rs/topcoat/pull/581))
- *(metadata)* built-in page metadata support ([#570](https://github.com/tokio-rs/topcoat/pull/570))

### Fixed

- *(asset)* resolve asset URLs with the view's context ([#596](https://github.com/tokio-rs/topcoat/pull/596))
