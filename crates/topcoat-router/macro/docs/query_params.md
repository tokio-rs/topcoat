Declares a typed view of the request's query string.

Apply this attribute to a struct with named fields. The macro derives [`serde::Deserialize`] on the struct so it can be read with the [`query_params::<T>(cx)`](fn.query_params.html) function, which parses the query string of whichever request `cx` belongs to using [`serde_urlencoded`].

The same struct can be used from any handler — it is not tied to a particular route. [`query_params`](fn.query_params.html) returns `Result<&T, &serde_urlencoded::de::Error>`, and parsing is memoized per request so repeated calls within one handler share the same parse result.

# Examples

```rust
use topcoat::{
    context::Cx,
    Result,
    router::{page, query_params},
    view::view,
};

#[query_params]
struct PageQuery {
    page: Option<u32>,
}

#[page]
async fn posts(cx: &Cx) -> Result {
    // For `/posts?page=2`, this yields `Some(2)`.
    let q = query_params::<PageQuery>(cx).unwrap();
    view! {
        <div>
            "currently on page: " (q.page)
        </div>
    }
}
```

# Requirements

- The struct's fields must be deserializable by `serde_urlencoded` (use `Option<T>` for optional parameters, since `serde_urlencoded` does not apply `#[serde(default)]` automatically).
- The struct must be `Send + Sync + 'static` to be memoized across the request.
