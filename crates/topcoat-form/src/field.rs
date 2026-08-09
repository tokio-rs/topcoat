use std::sync::Arc;

use crate::{
    ValidationEnv, ValidationError, Value,
    validator::{CustomValidator, Validator, ValidatorEntry},
};

/// A chain of validators for a single form field.
#[derive(Default)]
pub struct Field {
    validators: Vec<ValidatorEntry>,
    next_env: ValidationEnv,
}

impl Field {
    /// Creates an empty field with no validators.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses the raw input as a string.
    #[must_use]
    pub fn string(mut self) -> Self {
        self.add(StringValidator);
        self
    }

    /// Parses the raw input as a number.
    #[must_use]
    pub fn number(mut self) -> Self {
        self.add(NumberValidator);
        self
    }

    /// Parses the raw input as a boolean.
    #[must_use]
    pub fn bool(mut self) -> Self {
        self.add(BoolValidator);
        self
    }

    /// Requires the field to be present and non-empty.
    #[must_use]
    pub fn required(mut self) -> Self {
        self.add(RequiredValidator);
        self
    }

    /// Requires the string value to look like an email address.
    #[must_use]
    pub fn email(mut self) -> Self {
        self.add(EmailValidator);
        self
    }

    /// Requires a string value to have at least `length` characters.
    #[must_use]
    pub fn min_length(mut self, length: usize) -> Self {
        self.add(MinLengthValidator(length));
        self
    }

    /// Requires a string value to have at most `length` characters.
    #[must_use]
    pub fn max_length(mut self, length: usize) -> Self {
        self.add(MaxLengthValidator(length));
        self
    }

    /// Requires a numeric value to be at least `minimum`.
    #[must_use]
    pub fn min(mut self, minimum: f64) -> Self {
        self.add(MinValidator(minimum));
        self
    }

    /// Requires a numeric value to be at most `maximum`.
    #[must_use]
    pub fn max(mut self, maximum: f64) -> Self {
        self.add(MaxValidator(maximum));
        self
    }

    /// Requires a string value to be one of the given options.
    #[must_use]
    pub fn one_of(mut self, options: &'static [&'static str]) -> Self {
        self.add(OneOfValidator(options));
        self
    }

    /// Supplies a default value when the field is missing.
    #[must_use]
    pub fn or_default(mut self, value: impl Into<Value>) -> Self {
        self.add(DefaultValidator(value.into()));
        self
    }

    /// Adds a custom validator.
    #[must_use]
    pub fn custom<F>(mut self, f: F) -> Self
    where
        F: Fn(&Value, ValidationEnv) -> Result<Value, ValidationError> + Send + Sync + 'static,
    {
        self.add(CustomValidator::new(f));
        self
    }

    /// Runs subsequent validators only on the server.
    #[must_use]
    pub fn server_only(mut self) -> Self {
        self.next_env = ValidationEnv::Server;
        self
    }

    /// Runs subsequent validators only on the client.
    #[must_use]
    pub fn client_only(mut self) -> Self {
        self.next_env = ValidationEnv::Client;
        self
    }

    /// Runs subsequent validators on both the server and the client.
    #[must_use]
    pub fn both(mut self) -> Self {
        self.next_env = ValidationEnv::Both;
        self
    }

    fn add<V: Validator + 'static>(&mut self, validator: V) {
        self.validators.push(ValidatorEntry {
            validator: Arc::new(validator),
            env: self.next_env,
        });
    }

    pub(crate) fn validators(&self) -> &[ValidatorEntry] {
        &self.validators
    }
}

#[derive(Default)]
struct StringValidator;

impl Validator for StringValidator {
    fn validate(&self, value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
        match value {
            Value::Missing => Ok(Value::Missing),
            Value::String(_) => Ok(value.clone()),
            _ => Err("expected a string".into()),
        }
    }
}

#[derive(Default)]
struct NumberValidator;

impl Validator for NumberValidator {
    fn validate(&self, value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
        match value {
            Value::Missing => Ok(Value::Missing),
            Value::String(text) => text
                .parse::<f64>()
                .map(Value::Number)
                .map_err(|_| "expected a number".into()),
            Value::Number(_) => Ok(value.clone()),
            _ => Err("expected a number".into()),
        }
    }
}

#[derive(Default)]
struct BoolValidator;

impl Validator for BoolValidator {
    fn validate(&self, value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
        match value {
            Value::Missing => Ok(Value::Missing),
            Value::String(text) => parse_bool(text).map(Value::Bool),
            Value::Bool(_) => Ok(value.clone()),
            _ => Err("expected a boolean".into()),
        }
    }
}

#[derive(Default)]
struct RequiredValidator;

impl Validator for RequiredValidator {
    fn validate(&self, value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
        let missing = match value {
            Value::Missing => true,
            Value::String(text) => text.is_empty(),
            _ => false,
        };

        if missing {
            Err("is required".into())
        } else {
            Ok(value.clone())
        }
    }
}

#[derive(Default)]
struct EmailValidator;

impl Validator for EmailValidator {
    fn validate(&self, value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
        match value {
            Value::Missing => Ok(Value::Missing),
            Value::String(text) => {
                if is_valid_email(text) {
                    Ok(value.clone())
                } else {
                    Err("must be a valid email address".into())
                }
            }
            _ => Err("must be a valid email address".into()),
        }
    }
}

struct MinLengthValidator(usize);

impl Validator for MinLengthValidator {
    fn validate(&self, value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
        match value {
            Value::Missing => Ok(Value::Missing),
            Value::String(text) if text.chars().count() >= self.0 => Ok(value.clone()),
            Value::String(_) => Err(format!("must be at least {} characters", self.0).into()),
            _ => Err("expected a string".into()),
        }
    }
}

struct MaxLengthValidator(usize);

impl Validator for MaxLengthValidator {
    fn validate(&self, value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
        match value {
            Value::Missing => Ok(Value::Missing),
            Value::String(text) if text.chars().count() <= self.0 => Ok(value.clone()),
            Value::String(_) => Err(format!("must be at most {} characters", self.0).into()),
            _ => Err("expected a string".into()),
        }
    }
}

struct MinValidator(f64);

impl Validator for MinValidator {
    fn validate(&self, value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
        match value {
            Value::Missing => Ok(Value::Missing),
            Value::Number(number) if *number >= self.0 => Ok(value.clone()),
            Value::Number(_) => Err(format!("must be at least {}", self.0).into()),
            _ => Err("expected a number".into()),
        }
    }
}

struct MaxValidator(f64);

impl Validator for MaxValidator {
    fn validate(&self, value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
        match value {
            Value::Missing => Ok(Value::Missing),
            Value::Number(number) if *number <= self.0 => Ok(value.clone()),
            Value::Number(_) => Err(format!("must be at most {}", self.0).into()),
            _ => Err("expected a number".into()),
        }
    }
}

struct OneOfValidator(&'static [&'static str]);

impl Validator for OneOfValidator {
    fn validate(&self, value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
        match value {
            Value::Missing => Ok(Value::Missing),
            Value::String(text) if self.0.contains(&text.as_str()) => Ok(value.clone()),
            Value::String(_) => Err("is not an allowed value".into()),
            _ => Err("expected a string".into()),
        }
    }
}

struct DefaultValidator(Value);

impl Validator for DefaultValidator {
    fn validate(&self, value: &Value, _env: ValidationEnv) -> Result<Value, ValidationError> {
        match value {
            Value::Missing => Ok(self.0.clone()),
            _ => Ok(value.clone()),
        }
    }
}

fn parse_bool(text: &str) -> Result<bool, ValidationError> {
    match text.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err("expected a boolean".into()),
    }
}

fn is_valid_email(email: &str) -> bool {
    if email.len() > 254 {
        return false;
    }

    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };

    if local.is_empty() || local.len() > 64 {
        return false;
    }

    if domain.is_empty() || !domain.contains('.') {
        return false;
    }

    if domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }

    if email.contains(' ') {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_rejects_missing_and_empty() {
        assert!(Field::new().required()
            .validators()[0]
            .validator
            .validate(&Value::Missing, ValidationEnv::Both)
            .is_err());
        assert!(Field::new().required()
            .validators()[0]
            .validator
            .validate(&Value::String(String::new()), ValidationEnv::Both)
            .is_err());
        assert!(Field::new().required()
            .validators()[0]
            .validator
            .validate(&Value::String("hello".into()), ValidationEnv::Both)
            .is_ok());
    }

    #[test]
    fn email_validator_accepts_valid_email() {
        let field = Field::new().email();
        let validator = &field.validators()[0].validator;

        assert!(validator
            .validate(&Value::String("hello@example.com".into()), ValidationEnv::Both)
            .is_ok());
        assert!(validator
            .validate(&Value::String("not-an-email".into()), ValidationEnv::Both)
            .is_err());
    }

    #[test]
    fn number_validator_parses_and_checks_range() {
        let field = Field::new().number().min(18.0);
        let validators = field.validators();

        assert_eq!(
            validators[0].validator.validate(&Value::String("21".into()), ValidationEnv::Both),
            Ok(Value::Number(21.0))
        );
        assert!(validators[1]
            .validator
            .validate(&Value::Number(21.0), ValidationEnv::Both)
            .is_ok());
        assert!(validators[1]
            .validator
            .validate(&Value::Number(16.0), ValidationEnv::Both)
            .is_err());
    }

    #[test]
    fn server_only_validator_is_tagged_correctly() {
        let field = Field::new().string().server_only().required();
        let validators = field.validators();

        assert_eq!(validators[0].env, ValidationEnv::Both);
        assert_eq!(validators[1].env, ValidationEnv::Server);
    }

    #[test]
    fn default_fills_missing_value() {
        let field = Field::new().or_default(false);
        let validator = &field.validators()[0].validator;

        assert_eq!(
            validator.validate(&Value::Missing, ValidationEnv::Both),
            Ok(Value::Bool(false))
        );
    }
}
