use crate::{FormSchema, ValidationEnv};
use topcoat_core::error::Result;

/// Validates a form on the client using the type's schema.
///
/// `form_data` is a URL-encoded form string (typically produced by
/// `URLSearchParams(new FormData(...)).toString()`). On success an empty string
/// is returned; on failure a human-readable error message is returned.
pub async fn validate_client<T>(form_data: String) -> Result<String>
where
    T: FormSchema,
{
    let input: Vec<(String, String)> =
        form_urlencoded::parse(form_data.as_bytes()).into_owned().collect();

    match T::schema().validate(ValidationEnv::Client, &input) {
        Ok(_) => Ok(String::new()),
        Err(errors) => Ok(errors.to_string()),
    }
}
