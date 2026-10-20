Creates a module named `msg` for a set of [Fluent] files.

```rust
topcoat::l10n::fluent::messages!(pub(crate) "l10n/{locale}.ftl", fallback = "en");
```

All arguments are optional. A visibility before the path applies to the generated module.

The module contains a `CATALOG` static, a [`Catalog`](topcoat::l10n::Catalog) of the messages in every locale. Its [supported locales](topcoat::l10n::Catalog::supported_locales) are the locales that have files.

# Messages

Each message has a module named after its ID, with hyphens replaced by underscores, such as `msg::login_input` for `login-input`. An ID that is a Rust keyword becomes a raw identifier, such as `msg::r#type`.

The module holds an `Args` struct with a field for each variable the message uses in any locale, including the variables of the messages it references. Its `format` function returns a [`Translation`](topcoat::l10n::Translation), which renders in a view as the message in the locale of the scope, or turns into text with [`resolve`](topcoat::l10n::Translation::resolve):

```rust
# use topcoat::context::Cx;
topcoat::l10n::fluent::messages!("l10n/{locale}.ftl");

let hello = msg::hello::format(msg::hello::Args {});
assert_eq!(hello.resolve(&Cx::default()), "Hello, world!");
```

Each attribute of a message has a module within the message's module, such as `msg::login_input::placeholder`, with its own `Args` and `format`.

# Fallback

A locale does not need to translate every message. A message that a locale lacks is taken from the fallback locale. When there are files for more than one locale, choose the fallback with `fallback = "..."`. It must be one of the locales that have files.

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
