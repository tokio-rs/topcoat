use serde::de::DeserializeOwned;
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{Body, content::RawForm, error::bad_request, request::FromRequest};

use crate::{Schema, ValidationData, ValidationEnv, ValidationErrors};

/// A form extractor that validates its contents with a [`Schema`].
///
/// Implement [`FormSchema`] for a type that describes the expected fields, then
/// use `ValidForm<T>` as a handler parameter. The request body is deserialized
/// into `T` and then validated against [`T::schema`](FormSchema::schema).
///
/// ```rust
/// use serde::Deserialize;
/// use topcoat_validation::{Field, FormSchema, Schema, ValidForm, ValidationData, Value};
///
/// #[derive(Deserialize)]
/// struct SignUp {
///     email: String,
/// }
///
/// impl ValidationData for SignUp {
///     fn field(&self, name: &str) -> Option<Value> {
///         match name {
///             "email" => Some(Value::String(self.email.clone())),
///             _ => None,
///         }
///     }
/// }
///
/// impl FormSchema for SignUp {
///     fn schema() -> Schema {
///         Schema::new().field("email", Field::new().string().required().email())
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ValidForm<T: FormSchema>(pub T);

impl<T> FromRequest for ValidForm<T>
where
    T: FormSchema + DeserializeOwned + ValidationData,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        let RawForm(bytes) = RawForm::from_request(cx, body).await?;
        let value = serde_urlencoded::from_bytes::<T>(&bytes)
            .map_err(|error| bad_request(format!("invalid form: {error}")))?;
        T::schema()
            .validate(ValidationEnv::Server, &value)
            .map_err(|errors| bad_request(errors.to_string()))?;
        Ok(Self(value))
    }
}

/// A type that can be extracted from a validated form.
pub trait FormSchema: ValidationData + Send + Sync + Sized + 'static {
    /// Returns the validation schema for this form.
    fn schema() -> Schema;

    /// Validates this form value against [`Self::schema`] in the given
    /// environment.
    fn validate_with(&self, env: ValidationEnv) -> Result<(), ValidationErrors> {
        Self::schema().validate(env, self)?;
        Ok(())
    }

    /// Validates this form value on the server.
    fn validate_server(&self) -> Result<(), ValidationErrors> {
        self.validate_with(ValidationEnv::Server)
    }

    /// Validates this form value on the client.
    fn validate_client(&self) -> Result<(), ValidationErrors> {
        self.validate_with(ValidationEnv::Client)
    }
}
