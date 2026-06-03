# The `attributes!` macro

The `attributes!` macro builds a `topcoat::view::Attributes` value from Topcoat's attribute syntax.

Use it when attributes need to be passed around, assembled outside a `view!` call, changed at runtime, or forwarded through components.

```rust
use topcoat::view::{attributes, view};

let attrs = attributes! {
    class="button"
    type="submit"
    aria-label="Save changes"
};

view! {
    <button (attrs)>"Save"</button>
}
```

## Syntax

The body of `attributes!` has the same syntax as attributes inside an element in `view!`.

That includes literal attributes, expression values, dynamic names, binding attributes, event handlers, and attribute-level control flow:

```rust
use topcoat::view::attributes;

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
```

`attributes!` produces attributes, not child nodes. Control-flow bodies inside the macro therefore emit attributes in the same way they do inside a `view!` element's opening tag.

## Runtime Attributes

The generated value is `topcoat::view::Attributes`. It is a runtime collection of attributes with unique keys.

```rust
use topcoat::view::attributes;

let mut attrs = attributes! {
    class="button"
    data-state="idle"
};

attrs.insert("data-state", "loading");
attrs.insert("disabled", true);

assert!(attrs.contains_key("class"));
```

Because `Attributes` is map-like, each key appears at most once. Inserting the same key again replaces the previous value. Do not rely on render order for attributes.

## Inserting Attributes Into Elements

Insert an `Attributes` value into an element by using it as a parenthesized attribute fragment:

```rust
use topcoat::view::{attributes, view};

let attrs = attributes! {
    class="card"
    data-kind="summary"
};

view! {
    <article (attrs)>
        <h2>"Summary"</h2>
    </article>
}
```

Any type that implements `AttributeViewParts` can be used in the same position. `Attributes` implements that trait, so it works as a complete reusable attribute fragment.

Inserting an `Attributes` value consumes it. Clone the value first if the same attribute collection needs to be inserted into more than one element.

## Passing Attributes To Components

Components can accept `Attributes` as a normal argument. This is useful for forwarding caller-controlled attributes to the component's root element.

```rust
use topcoat::{
    Result,
    view::{Attributes, View, attributes, component, view},
};

#[component]
async fn panel(attrs: Attributes, child: View) -> Result {
    view! {
        <section (attrs)>
            (child)
        </section>
    }
}

view! {
    panel(
        attrs: attributes! {
            class="panel"
            data-panel="account"
        },
        <p>"Account settings"</p>
    )
}
```

Since the value is ordinary Rust data, you can build it in helper functions, add or replace attributes before rendering, and pass it through several layers before inserting it into an element.
