use topcoat::{
    Result,
    router::{content::multipart::Multipart, route},
};

// Multipart streams multipart/form-data fields, commonly used for file uploads.
// Available with the `multipart` feature.
#[route(POST)]
pub(crate) async fn files(mut multipart: Multipart) -> Result<String> {
    let mut total = 0;

    while let Some(field) = multipart.next_field().await? {
        let name = field.name().map(str::to_owned);
        let data = field.bytes().await?;

        println!("field {name:?}: {} bytes", data.len());
        total += data.len();
    }

    Ok(format!("received {total} bytes across all fields"))
}
