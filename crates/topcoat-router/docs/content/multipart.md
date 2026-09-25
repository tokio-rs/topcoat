Multipart form data for Topcoat routes.

Enable the `multipart` feature to read `multipart/form-data`, the format used by browser forms that upload files. The [`Multipart`] extractor yields each form field as a [`Field`].

# Reading fields

Accept a [`Multipart`] parameter and call [`next_field`](Multipart::next_field) to read fields in request order. Finish reading or drop each field before requesting the next one.

```rust
use topcoat::{
    Result,
    router::{content::multipart::Multipart, route},
};

#[route(POST "/api/upload")]
async fn upload(mut multipart: Multipart) -> Result<&'static str> {
    while let Some(field) = multipart.next_field().await? {
        let name = field.name().map(str::to_owned);
        let data = field.bytes().await?;

        println!("field `{name:?}` is {} bytes", data.len());
    }

    Ok("received")
}
```

Use a [`Field`]'s accessors to inspect its metadata. Read all its data with [`bytes`](Field::bytes) or [`text`](Field::text), or read incrementally with [`chunk`](Field::chunk). A field also implements `Stream`.

Wrap the extractor in [`Option`] to make the body optional: the route then also accepts requests without a `multipart/form-data` body. A request whose multipart body is malformed is rejected with `400 Bad Request`.
