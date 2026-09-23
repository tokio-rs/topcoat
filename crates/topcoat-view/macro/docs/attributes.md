The [`attributes!`] macro builds an [`Attributes`] collection from the same attribute syntax that [`view!`] uses inside an opening tag.

Use it when attributes need to be built outside of a [`view!`] call, changed at run time, or passed through components.

```rust
# #[topcoat::view::component]
# async fn example() -> topcoat::Result<impl topcoat::view::View> {
use topcoat::view::{View, attributes, view};

let attrs = attributes! {
    class="button"
    type="submit"
    aria-label="Save changes"
};

Ok(view! {
    <button (attrs)>"Save"</button>
})
# }
```

# Syntax

The body of [`attributes!`] is written exactly like the attributes in an opening tag in [`view!`]. Literal values, expression values, expression names, bind attributes, event handlers, inserted collections, and `if`/`for`/`match`/`let` all work the same way:

```rust
# #[topcoat::view::component]
# async fn example() -> topcoat::Result<impl topcoat::view::View> {
use topcoat::view::{View, attributes, view};

let id = "submit";
let extra = [
    ("data-state", "ready"),
    ("data-size", "compact"),
];

let attrs = attributes! {
    class="button"
    id=(id)
    :data-bound=$(id.to_owned())
    @input="(e) => console.log(e)"

    if id == "submit" {
        type="submit"
    } else {
        type="button"
    }

    for (name, value) in extra {
        (name)=(value)
    }

    match id {
        "submit" => aria-label="Submit",
        _ => aria-label="Button",
    }
};
# Ok(view! { <button (attrs)></button> })
# }
```

As in an opening tag, writing the same literal attribute name twice is a compile error.

Inside a [`component`], `#[page]`, or `#[layout]`, the request context is in scope implicitly. In a plain function, name the context at the start of the body, as in `attributes! { cx => class="button" }`, the same way as with [`view!`].

# Runtime Attributes

[`attributes!`] evaluates to an [`Attributes`] value: a collection of attributes with unique names that you can read and change at run time.

```rust
# use topcoat::{Result, context::Cx, view::{View, component, view}};
# #[component]
# async fn example(cx: &Cx) -> Result<impl View> {
use topcoat::view::attributes;

let mut attrs = attributes! {
    class="button"
    data-state="idle"
};

attrs.insert(cx, "data-state", "loading");
attrs.insert(cx, "disabled", true);

assert!(attrs.contains_key("class"));
# Ok(view! { <div (attrs)></div> })
# }
```

[`Attributes`] works like a map: each name appears at most once, and inserting a name again replaces its previous value. The attributes do not render in a fixed order.

# Inserting Attributes Into Elements

Insert an [`Attributes`] value into an element by writing it in parentheses inside the opening tag:

```rust
# #[topcoat::view::component]
# async fn example() -> topcoat::Result<impl topcoat::view::View> {
use topcoat::view::{View, attributes, view};

let attrs = attributes! {
    class="card"
    data-kind="summary"
};

Ok(view! {
    <article (attrs)>
        <h2>"Summary"</h2>
    </article>
})
# }
```

When the element has other attributes too, a later attribute replaces an earlier one with the same name. For example, in `<div class="card" (attrs)>` a `class` in `attrs` wins, and in `<div (attrs) class="card">` the literal wins.

Any type that implements [`AttributeViewParts`] can be inserted the same way. [`Attributes`] is one such type.

Inserting an [`Attributes`] value consumes it. Clone it first if you need to insert the same collection into more than one element.

# Passing Attributes To Components

A component can take [`Attributes`] as an ordinary parameter. This lets callers choose attributes for one of the component's elements:

```rust
use topcoat::{
    Result,
    view::{Attributes, Child, View, attributes, component, view},
};

#[component]
async fn panel(attrs: Attributes, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <section (attrs)>
            (child)
        </section>
    })
}

# #[topcoat::view::component]
# async fn example() -> Result<impl View> {
Ok(view! {
    panel(
        attrs: attributes! {
            class="panel"
            data-panel="account"
        },
        <p>"Account settings"</p>
    )
})
# }
```

Since the value is ordinary Rust data, you can build it in helper functions, add or replace attributes before rendering, and pass it through several layers of components before inserting it into an element.

[`AttributeViewParts`]: trait.AttributeViewParts.html
[`Attributes`]: struct.Attributes.html
[`attributes!`]: macro.attributes.html
[`component`]: attr.component.html
[`view!`]: macro.view.html
