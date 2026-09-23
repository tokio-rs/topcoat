/// Properties with a builder that checks required fields at compile time.
///
/// Implemented by [`#[derive(Props)]`][derive]. The derive also generates an
/// inherent `builder()` function on the struct, so this trait only needs to be
/// in scope when the builder is constructed generically.
///
/// [derive]: derive.Props.html
pub trait Props {
    /// The builder type in its initial state, with no properties set.
    type Builder;

    /// Returns a builder with no properties set.
    fn builder() -> Self::Builder;
}

/// Marks a required property as set in a generated builder.
pub struct Set;

/// Marks a builder property as ready to build.
///
/// A generated builder's `build()` method requires this trait for every
/// required property.
#[diagnostic::on_unimplemented(
    message = "missing required property `{Self}`",
    label = "the property `{Self}` was not set",
    note = "every property without `#[default]` must be set before calling `build()`"
)]
pub trait IsSet {}

impl IsSet for Set {}
