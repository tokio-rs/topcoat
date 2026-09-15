/// Field validation for request input.
///
/// Parse errors (malformed JSON, a wrong form shape) are rejected with
/// `400 Bad Request` by the extractors themselves. Validation covers the next
/// step: a well formed value that breaks a domain rule, like an empty title
/// or an out of range page number. Implement [`Validate`] for the input type,
/// call it from the handler, and return the collected [`ValidationErrors`]
/// on failure. The router renders them as `422 Unprocessable Entity` with a
/// JSON body, or the handler re-renders the form with them inline.
///
/// ```rust
/// use topcoat::validation::{Validate, ValidationErrors, rules};
///
/// struct NewTodo {
///     title: String,
/// }
///
/// impl Validate for NewTodo {
///     fn validate(&self) -> Result<(), ValidationErrors> {
///         let mut errors = ValidationErrors::new();
///         errors.check(
///             "title",
///             rules::required(&self.title),
///             "title cannot be empty",
///         );
///         errors.check(
///             "title",
///             rules::max_length(&self.title, 120),
///             "title must be at most 120 characters",
///         );
///         errors.into_result()
///     }
/// }
/// ```
mod error;
pub mod rules;
mod validate;

pub use error::*;
pub use validate::*;
