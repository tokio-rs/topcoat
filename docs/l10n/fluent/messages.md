Loads [Fluent] files and creates `t!` to translate their messages.

```rust
topcoat::l10n::fluent::messages!(pub(crate) "l10n/{locale}.ftl");
```

The argument is the path of the files, with a `{locale}` placeholder. A visibility before the path applies to `t!` and to the `t` module. A `pub` invocation exports `t!` at the root of the crate, so other crates can use it.

# Translating

`t!` takes the ID of a message, such as `t!(login-input)`. Follow it with `.name` to translate one of the message's attributes, such as `t!(login-input.placeholder)`. An ID that is not made of Rust tokens can be written as a string literal, such as `t!("item-1e")`.

After the ID, pass the variables of the message as `name = value`, or as `name` for a value in a variable of the same name. A value can be a string, such as a `&str` or a `String`, or a number of any primitive type. Borrowed values are not copied.

`t!` returns a [`Translation`]. Used in a view, as content or as an attribute value, it renders as the message in the locale of the scope. Outside a view, turn it into text with [`resolve`] or [`resolve_in`]:

```rust
# use topcoat::{context::Cx, l10n::locale};
topcoat::l10n::fluent::messages!("l10n/{locale}.ftl");

let cx = &Cx::default().with(locale!("en"));
assert_eq!(t!(hello).resolve(cx), "Hello, world!");
assert_eq!(t!(login-input.placeholder).resolve(cx), "Your name");

let name = "Ada";
let greeting = t!(greeting, user-name = name).resolve(cx);
```

When the locale has no files, or its files lack the message, the translation renders as a placeholder made of the key and the arguments, such as `greeting(user-name = "Ada")`. `t!` does not read the files, so it does not check the key or the arguments. `topcoat l10n check` does.

# The `t` module

The `t` module holds `t::locales()`, which returns the locales that have files, sorted by their spelling. Pass it to [`negotiate`] to pick the locale of a request.

# Paths

The path contains exactly one `{locale}` placeholder, which matches a locale within a single path component. A path starting with `./` or `../` is relative to the invoking source file, a path starting with `/` is absolute, and any other path is relative to the crate's `Cargo.toml`.

The path can also start with `{mod}`, which stands for the directory of the invoking module. That is the directory where its child modules live, such as `src/about/` for both `src/about.rs` and `src/about/mod.rs`.

The path can contain these wildcards:

- `*` matches any characters within a single path component. It cannot appear in the same path component as `{locale}`.
- `**` matches zero or more directories and must be a whole path component.

Other braces, `?`, `[` and `]` are not allowed. Wildcards and `{locale}` do not match names starting with `.` unless the pattern does.

So `"./l10n/{locale}.ftl"` matches one file per locale, such as `l10n/en.ftl`, and `"./l10n/{locale}/**/*.ftl"` matches every file below a directory per locale, such as `l10n/en/forms/login.ftl`.

# Locales

A locale takes the messages it lacks from the locales it falls back to, so `de-CH` only needs the messages that differ from `de`. It never takes messages from another language.

These are compile errors:

- no file matches the path,
- the text matched by `{locale}` is not a valid locale,
- two files name the same locale, such as `zh` and `zh-Hans`,
- a file has a Fluent syntax error,
- a message or term is defined more than once in the files of one locale.

The crate is rebuilt when a matched file changes. Cargo does not notice a new file that starts to match, so it is only included once another file of the crate changes. `topcoat dev` rebuilds the crate when a file is added.

[Fluent]: https://projectfluent.org/
[`Translation`]: ../struct.Translation.html
[`resolve`]: ../struct.Translation.html#method.resolve
[`resolve_in`]: ../struct.Translation.html#method.resolve_in
[`negotiate`]: ../fn.negotiate.html
