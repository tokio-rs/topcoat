Declares a typed view of the request's query string.

Apply `#[query_params]` to a struct with named fields. The macro derives [`serde::Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html), so each field maps to a key in the query string. Read the values with [`query_params::<T>(cx)`](fn.query_params.html) from any handler.

```rust
# use topcoat::router::query_params;
#[query_params]
struct PageQuery {
    page: Option<u32>,
}
```

# Reading the value

[`query_params::<T>(cx)`](fn.query_params.html) parses the current request's query string with [`serde_urlencoded`](https://docs.rs/serde_urlencoded/latest/serde_urlencoded/) and returns `Result<&T, &serde_urlencoded::de::Error>`. Unlike a path parameter, the struct is not tied to a route: any handler can read it. Parsing runs at most once per request; the result is then memoized.

# Example

```rust
use topcoat::{
    context::Cx,
    Result,
    router::{RouterErrorExt, page, query_params},
    view::view,
};

#[query_params]
struct PageQuery {
    page: Option<u32>,
}

#[page("/posts")]
async fn posts(cx: &Cx) -> Result {
    // For `/posts?page=2`, this yields `Some(2)`.
    let q = query_params::<PageQuery>(cx).ok_or_bad_request("invalid query string")?;
    view! {
        <div>
            "currently on page: " (q.page)
        </div>
    }
}
```

# Requirements

- Use `Option<T>` for optional keys. `serde_urlencoded` does not apply `#[serde(default)]`, so a missing key on a non-`Option` field is a parse error.
- The struct must be `Send + Sync + 'static`, since it is memoized for the request.
