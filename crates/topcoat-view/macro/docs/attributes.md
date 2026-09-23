The [`attributes!`] macro builds a [`topcoat::view::Attributes`] value from Topcoat's attribute syntax.

Use it to prepare attributes before inserting them into an element:

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

The body of [`attributes!`] has the same syntax as attributes inside an element in [`view!`].

For example, use Rust control flow to choose attributes:

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

Control-flow bodies produce attributes. See the [`view!`] guide for the full syntax.

# Runtime Attributes

You can change the resulting collection before rendering it:

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

Each key appears at most once. Inserting the same key again replaces its value. Attribute render order is unspecified.

# Inserting Attributes Into Elements

Insert an [`Attributes`] value into an element by using it as a parenthesized attribute fragment:

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

Implement [`AttributeViewParts`] to insert your own attribute collection in this position.

Inserting an [`Attributes`] value consumes it. Clone the value first if the same attribute collection needs to be inserted into more than one element.

# Passing Attributes To Components

Components can accept [`Attributes`] as a normal argument. This is useful for forwarding caller-controlled attributes to one of the component's HTML elements.

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


[`AttributeViewParts`]: trait.AttributeViewParts.html
[`Attributes`]: struct.Attributes.html
[`attributes!`]: macro.attributes.html
[`topcoat::view::Attributes`]: struct.Attributes.html
[`view!`]: macro.view.html
