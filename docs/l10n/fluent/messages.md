Load translations from [Fluent] files and use the generated `t!` macro to display them.

```rust
topcoat::l10n::fluent::messages!(pub(crate) "l10n/{locale}.ftl");
```

Pass a file path with `{locale}` where the locale name belongs. You can put a visibility such as `pub(crate)` before the path. It applies to both `t!` and the generated `t` module. With `pub`, the macro is exported at the crate root for use by other crates.

# Translating

To translate a message, pass its ID to `t!`, as in `t!(login-input)`. To translate an attribute, add its name after a dot, as in `t!(login-input.placeholder)`. Use a string literal if Rust cannot read the ID as tokens, as in `t!("item-1e")`.

Supply message variables after the ID with `name: value`. If the Rust variable has the same name, you can write just `name`. Values can be strings (`&str` or `String`) or primitive numbers. Borrowed values stay borrowed. For other types, implement [`FluentType`] and pass [`Argument::custom`].

rustfmt reads a hyphen as a minus sign, so it formats `t!(login-input)` as `t!(login - input)`. The macro still finds the right message. To keep the hyphens together, write the key as a string literal, as in `t!("login-input.placeholder")`. You can also avoid hyphens by using snake case IDs such as `login_input` in your Fluent files. Argument names can contain hyphens, as in `t!(greeting, user-name: name)`, or be string literals, as in `t!(greeting, "user-name": name)`.

The result is a [`Translation`]. In a view, you can use it as text or as an attribute value. It uses the locale in the current context. Call [`resolve`] or [`resolve_in`] when you need the text itself:

```rust
# use topcoat::{context::Cx, l10n::locale};
topcoat::l10n::fluent::messages!("l10n/{locale}.ftl");

let cx = &Cx::new().with(locale!("en"));
assert_eq!(t!(hello).resolve(cx), "Hello, world!");
assert_eq!(t!(login-input.placeholder).resolve(cx), "Your name");

let name = "Ada";
let greeting = t!(greeting, user-name: name).resolve(cx);
```

If neither the locale nor its fallback locales provide the message, the result shows the key and arguments, such as `greeting(user-name: "Ada")`. The `t!` macro does not check keys or arguments against the files. Run `topcoat l10n check` to check them.

# The `t` module

Call `t::locales()` to get the locales loaded from the files, sorted by their canonical names. Pass this list to [`negotiate`] to choose a request's locale.

# Paths

Include exactly one `{locale}` placeholder in the path. It matches a locale name within one file or directory name. Paths starting with `./` or `../` are relative to the source file containing the macro call. Paths starting with `/` are absolute. All other paths are relative to the directory containing the crate's `Cargo.toml`.

Start a path with `{mod}` to use the directory where the current module's child modules belong. For both `src/about.rs` and `src/about/mod.rs`, `{mod}` means `src/about/`.

Two wildcards are supported:

- `*` matches any characters within one file or directory name. It cannot share that name with `{locale}`.
- `**` matches any number of directories, including none. It must occupy a whole path component.

Other braces and the characters `?`, `[` and `]` are not supported. To match a name starting with `.`, the pattern for that name must also start with `.`.

For one file per locale, use `"./l10n/{locale}.ftl"`, which matches files such as `l10n/en.ftl`. For several files per locale, use `"./l10n/{locale}/**/*.ftl"`, which also finds files in subdirectories, such as `l10n/en/forms/login.ftl`.

# Locales

A locale uses its fallback locales for missing messages. For example, `de-CH` can use messages from `de`, so it only needs to define the messages that differ. Fallback does not cross language boundaries.

Compilation fails if:

- the path matches no files,
- a name matched by `{locale}` is invalid,
- different locale names refer to the same locale, such as `zh` and `zh-Hans`,
- a Fluent file contains invalid syntax,
- a locale defines the same message or term more than once.

Changes to matched files trigger a rebuild. Newly added files do not trigger Cargo on their own, so a plain Cargo build picks them up only after another file in the crate changes. `topcoat dev` triggers a rebuild when you add a file. `topcoat l10n check` forces the message files to be loaded again before checking them.

[Fluent]: https://projectfluent.org/
[`Translation`]: ../struct.Translation.html
[`resolve`]: ../struct.Translation.html#method.resolve
[`resolve_in`]: ../struct.Translation.html#method.resolve_in
[`negotiate`]: ../fn.negotiate.html
[`FluentType`]: ../trait.FluentType.html
[`Argument::custom`]: ../struct.Argument.html#method.custom
