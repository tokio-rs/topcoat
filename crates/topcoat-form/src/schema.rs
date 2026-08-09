use std::collections::HashMap;

use crate::{Field, ValidationData, ValidationEnv, ValidationErrors, ValidationResult, Value};

/// A validation schema describing one or more form fields.
#[derive(Default)]
pub struct Schema {
    fields: Vec<FieldRule>,
}

struct FieldRule {
    name: String,
    field: Field,
}

impl Schema {
    /// Creates an empty schema.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a field named `name` validated by `field`.
    #[must_use]
    pub fn field(mut self, name: impl Into<String>, field: Field) -> Self {
        self.fields.push(FieldRule {
            name: name.into(),
            field,
        });
        self
    }

    /// Validates `input` in `env`, returning a map of field names to their
    /// validated values.
    ///
    /// Validators configured for an environment other than `env` are skipped.
    /// A validator configured for [`Both`](ValidationEnv::Both) always runs,
    /// and a validation run configured as [`Both`](ValidationEnv::Both) runs
    /// every validator.
    ///
    /// # Errors
    ///
    /// Returns a [`ValidationErrors`] collection when any field fails
    /// validation.
    pub fn validate(
        &self,
        env: ValidationEnv,
        input: &impl ValidationData,
    ) -> ValidationResult<HashMap<String, Value>> {
        let mut output = HashMap::with_capacity(self.fields.len());
        let mut errors = ValidationErrors::new();

        for rule in &self.fields {
            let mut value = input.field(&rule.name).unwrap_or(Value::Missing);

            for entry in rule.field.validators() {
                if !entry.env.includes(env) {
                    continue;
                }

                match entry.validator.validate(&value, env) {
                    Ok(new_value) => value = new_value,
                    Err(error) => {
                        errors.push(rule.name.clone(), error);
                        break;
                    }
                }
            }

            if errors.get(&rule.name).is_none() {
                output.insert(rule.name.clone(), value);
            }
        }

        if errors.is_empty() {
            Ok(output)
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn valid_input_returns_values() {
        let schema = Schema::new()
            .field("email", Field::new().string().required().email())
            .field("age", Field::new().number().required().min(18.0));

        let input = HashMap::from([
            ("email".to_owned(), "hello@example.com".to_owned()),
            ("age".to_owned(), "21".to_owned()),
        ]);

        let values = schema.validate(ValidationEnv::Server, &input).unwrap();

        assert_eq!(
            values.get("email"),
            Some(&Value::String("hello@example.com".into()))
        );
        assert_eq!(values.get("age"), Some(&Value::Number(21.0)));
    }

    #[test]
    fn missing_required_field_collects_error() {
        let schema = Schema::new()
            .field("email", Field::new().string().required().email())
            .field("age", Field::new().number().required().min(18.0));

        let input = HashMap::from([("age".to_owned(), "16".to_owned())]);

        let errors = schema.validate(ValidationEnv::Server, &input).unwrap_err();

        assert_eq!(errors.get("email").map(|e| e.message()), Some("is required"));
    }

    #[test]
    fn server_only_validator_skipped_on_client() {
        let schema = Schema::new().field(
            "name",
            Field::new().string().server_only().min_length(10),
        );

        let input = HashMap::from([("name".to_owned(), "short".to_owned())]);

        assert!(schema.validate(ValidationEnv::Client, &input).is_ok());
        assert!(schema.validate(ValidationEnv::Server, &input).is_err());
    }

    #[test]
    fn client_only_validator_skipped_on_server() {
        let schema = Schema::new().field(
            "name",
            Field::new().string().client_only().min_length(10),
        );

        let input = HashMap::from([("name".to_owned(), "short".to_owned())]);

        assert!(schema.validate(ValidationEnv::Server, &input).is_ok());
        assert!(schema.validate(ValidationEnv::Client, &input).is_err());
    }

    #[test]
    fn defaults_fill_missing_values() {
        let schema = Schema::new().field(
            "newsletter",
            Field::new().bool().or_default(false),
        );

        let values = schema
            .validate(ValidationEnv::Both, &HashMap::<String, String>::new())
            .unwrap();

        assert_eq!(values.get("newsletter"), Some(&Value::Bool(false)));
    }
}
