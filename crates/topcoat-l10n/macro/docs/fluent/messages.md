Creates a module named `msg` for a set of [Fluent] files.

```rust
topcoat::l10n::fluent::messages!(pub(crate) "l10n/{locale}.ftl");
```

Both arguments are optional. A visibility before the path applies to the generated module.

# Paths

The path contains exactly one `{locale}` placeholder, which matches a locale within a single path component. A path starting with `./` or `../` is relative to the invoking source file, a path starting with `/` is absolute, and any other path is relative to the crate's `Cargo.toml`.

The path can also start with `{mod}`, which stands for the directory of the invoking module. That is the directory where its child modules live, such as `src/about/` for both `src/about.rs` and `src/about/mod.rs`. Without a path, the macro uses `"{mod}/l10n/{locale}.ftl"`.

The path can contain these wildcards:

- `*` matches any characters within a single path component. It cannot appear in the same path component as `{locale}`.
- `**` matches zero or more directories and must be a whole path component.

Other braces, `?`, `[` and `]` are not allowed. Wildcards and `{locale}` do not match names starting with `.` unless the pattern does.

Each locale can have several files. A message or term can only be defined once per locale. The crate is rebuilt when a matched file changes, but not when a new file starts to match.

# Renaming the module

To give the module another name, invoke the macro inside a module of your own and re-export its contents:

```rust
pub mod strings {
    topcoat::l10n::fluent::messages!("l10n/{locale}.ftl");
    pub use msg::*;
}
```

[Fluent]: https://projectfluent.org/
