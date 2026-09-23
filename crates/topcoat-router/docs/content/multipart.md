Multipart form data for Topcoat routes.

`multipart/form-data` is the format browsers use for forms that upload files. This module needs the `multipart` feature. It provides the [`Multipart`] extractor, which parses the request body and yields each form field as a [`Field`] that streams its data.

# Reading fields

To read an upload, take a [`Multipart`] parameter and loop over its fields with [`next_field`](Multipart::next_field). Fields arrive in the order of the request and are read one at a time. Each field borrows the extractor mutably, so you must finish with one field before you ask for the next.

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

A [`Field`] gives you its metadata through [`name`](Field::name), [`file_name`](Field::file_name), [`content_type`](Field::content_type), and [`headers`](Field::headers). You can read its data all at once with [`bytes`](Field::bytes) or [`text`](Field::text), or one chunk at a time with [`chunk`](Field::chunk). A field also implements `Stream`, so you can use the usual stream combinators on its chunks.

Wrap the extractor in [`Option`] to make the body optional. The route then also accepts requests without a `multipart/form-data` body. A malformed multipart body is rejected with `400 Bad Request`, and a body larger than the request's body limit with `413 Content Too Large`.
