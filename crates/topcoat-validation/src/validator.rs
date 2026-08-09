use std::sync::Arc;

use crate::{ValidationEnv, ValidationError, Value};

type ValidateFn =
    Arc<dyn Fn(&Value, ValidationEnv) -> Result<Value, ValidationError> + Send + Sync>;

/// A single validation step, paired with the environment it runs in.
pub(crate) struct ValidatorEntry {
    pub(crate) validator: Arc<dyn Validator>,
    pub(crate) env: ValidationEnv,
}

/// A validator transforms a [`Value`] into another value or fails with a
/// [`ValidationError`].
pub trait Validator: Send + Sync {
    /// Validates `value` and returns the transformed value.
    ///
    /// Validators should return [`Value::Missing`] unchanged when they describe
    /// a constraint on a present value, so optional fields stay optional unless
    /// a [`required`](super::Field::required) validator is also chained.
    ///
    /// # Errors
    ///
    /// Returns a [`ValidationError`] describing why the value failed.
    fn validate(&self, value: &Value, env: ValidationEnv) -> Result<Value, ValidationError>;
}

impl<V> Validator for Arc<V>
where
    V: Validator + ?Sized,
{
    fn validate(&self, value: &Value, env: ValidationEnv) -> Result<Value, ValidationError> {
        (**self).validate(value, env)
    }
}

/// A validator built from a closure.
pub struct CustomValidator {
    f: ValidateFn,
}

impl CustomValidator {
    /// Creates a validator from a closure.
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&Value, ValidationEnv) -> Result<Value, ValidationError> + Send + Sync + 'static,
    {
        Self { f: Arc::new(f) }
    }
}

impl Validator for CustomValidator {
    fn validate(&self, value: &Value, env: ValidationEnv) -> Result<Value, ValidationError> {
        (self.f)(value, env)
    }
}
