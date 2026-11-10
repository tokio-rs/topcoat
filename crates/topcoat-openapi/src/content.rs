use schemars::{JsonSchema, Schema, SchemaGenerator};
use topcoat_router::content::{Form, Json};

/// A request or response body that an API document can describe.
///
/// A type names its media type and the schema of its payload. Pass such a
/// type to [`Operation::request`](crate::Operation::request) or
/// [`Operation::response_with`](crate::Operation::response_with).
///
/// Implement it for your own extractors and response types:
///
/// ```
/// use topcoat::openapi::{
///     Content,
///     schemars::{Schema, SchemaGenerator, json_schema},
/// };
///
/// struct Csv(String);
///
/// impl Content for Csv {
///     const MEDIA_TYPE: &'static str = "text/csv";
///
///     fn schema(_generator: &mut SchemaGenerator) -> Schema {
///         json_schema!({ "type": "string" })
///     }
/// }
/// ```
pub trait Content {
    /// The media type of the body, such as `application/json`.
    const MEDIA_TYPE: &'static str;

    /// Whether a request has to carry this body.
    const REQUIRED: bool = true;

    /// Returns the schema of the payload.
    ///
    /// Call [`SchemaGenerator::subschema_for`] for a payload type that
    /// implements [`JsonSchema`], so the document lists the type once and
    /// refers to it from every body that uses it.
    fn schema(generator: &mut SchemaGenerator) -> Schema;
}

impl<T: JsonSchema> Content for Json<T> {
    const MEDIA_TYPE: &'static str = "application/json";

    fn schema(generator: &mut SchemaGenerator) -> Schema {
        generator.subschema_for::<T>()
    }
}

impl<T: JsonSchema> Content for Form<T> {
    const MEDIA_TYPE: &'static str = "application/x-www-form-urlencoded";

    fn schema(generator: &mut SchemaGenerator) -> Schema {
        generator.subschema_for::<T>()
    }
}

impl Content for String {
    const MEDIA_TYPE: &'static str = "text/plain";

    fn schema(generator: &mut SchemaGenerator) -> Schema {
        generator.subschema_for::<String>()
    }
}

impl Content for &str {
    const MEDIA_TYPE: &'static str = "text/plain";

    fn schema(generator: &mut SchemaGenerator) -> Schema {
        generator.subschema_for::<String>()
    }
}

/// A body that a request may leave out.
impl<T: Content> Content for Option<T> {
    const MEDIA_TYPE: &'static str = T::MEDIA_TYPE;
    const REQUIRED: bool = false;

    fn schema(generator: &mut SchemaGenerator) -> Schema {
        T::schema(generator)
    }
}

/// Generates a schema, adding the types it refers to to the generator.
pub(crate) type SchemaFn = fn(&mut SchemaGenerator) -> Schema;

/// A [`Content`] type, kept without its type parameter.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ContentType {
    pub(crate) media_type: &'static str,
    pub(crate) required: bool,
    pub(crate) schema: SchemaFn,
}

impl ContentType {
    /// Describes the body type `T`.
    pub(crate) fn of<T: Content>() -> Self {
        Self {
            media_type: T::MEDIA_TYPE,
            required: T::REQUIRED,
            schema: T::schema,
        }
    }
}
