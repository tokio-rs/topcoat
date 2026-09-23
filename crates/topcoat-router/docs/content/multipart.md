Multipart form data for topcoat routes.

`multipart/form-data` is the request format browsers use for forms that upload files. This module (behind the `multipart` feature) provides the [`Multipart`] extractor: it parses the request body and yields each form field as a [`Field`] that streams its data.

# Reading fields

A route reads an upload by taking a [`Multipart`] parameter and iterating its fields with [`next_field`](Multipart::next_field). Fields arrive in request order and are read one at a time; the extractor is borrowed mutably, so each field is consumed before the next one starts.

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

A [`Field`] exposes its metadata ([`name`](Field::name), [`file_name`](Field::file_name), [`content_type`](Field::content_type), [`headers`](Field::headers)) and its data, either in full with [`bytes`](Field::bytes) and [`text`](Field::text) or chunk by chunk with [`chunk`](Field::chunk). A field also implements `Stream`, so its chunks compose with the usual stream combinators.

Wrap the extractor in [`Option`] to make the body optional: the route then also accepts requests without a `multipart/form-data` body. A request whose multipart body is malformed is rejected with `400 Bad Request`.
