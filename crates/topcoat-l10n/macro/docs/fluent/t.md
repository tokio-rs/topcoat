Translates a message of a [`messages!`] module.

```rust
use topcoat::{
    Result,
    l10n::fluent::{messages, t},
    view::{View, component, view},
};

messages!("l10n/{locale}.ftl");

#[component]
async fn login(user_name: &str) -> Result<impl View> {
    Ok(view! {
        <p>(t!(msg::greeting, user_name))</p>
        <input placeholder=(t!(msg::login_input.placeholder))>
    })
}
```

The first argument is the path to the message's module, such as `msg::greeting`. Follow it with `.name` to translate one of the message's attributes, such as `msg::login_input.placeholder`.

Used in a view, as content or as an attribute value, the message renders in the locale of the scope. Outside a view, [`resolve`] the resulting [`Translation`] into text:

```rust
# use topcoat::{context::Cx, l10n::fluent::{messages, t}};
# messages!("l10n/{locale}.ftl");
# let cx = &Cx::default();
let text = t!(msg::login_input.placeholder).resolve(cx);
assert_eq!(text, "Your name");
```

# Arguments

After the path, pass a value for every variable of the message as `name = value`, or as `name` for a value in a variable of the same name. Hyphens in Fluent variable names become underscores, so `$user-name` is passed as `user_name`.

A missing or unknown argument is a compile error. A value can be a string, such as a `&str` or a `String`, a number of any primitive type, or an `Option` of either. Borrowed values are not copied.

[`messages!`]: macro.messages.html
[`Translation`]: ../struct.Translation.html
[`resolve`]: ../struct.Translation.html#method.resolve
