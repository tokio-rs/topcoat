use std::fmt::{self, Display};

/// A single validation failure for one field.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    message: String,
}

impl ValidationError {
    /// Creates a new validation error.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the error message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ValidationError {}

impl From<String> for ValidationError {
    fn from(message: String) -> Self {
        Self::new(message)
    }
}

impl From<&str> for ValidationError {
    fn from(message: &str) -> Self {
        Self::new(message)
    }
}

/// A collection of validation errors, keyed by field name.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ValidationErrors {
    errors: Vec<(String, ValidationError)>,
}

/// The result of running a validation schema.
pub type ValidationResult<T> = Result<T, ValidationErrors>;

impl ValidationErrors {
    /// Creates an empty error collection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an error for `field`.
    pub fn push(&mut self, field: impl Into<String>, error: impl Into<ValidationError>) {
        self.errors.push((field.into(), error.into()));
    }

    /// Returns `true` if there are no errors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns the number of recorded errors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Returns the first error for `field`, if any.
    #[must_use]
    pub fn get(&self, field: &str) -> Option<&ValidationError> {
        self.errors
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, error)| error)
    }

    /// Returns all recorded errors.
    #[must_use]
    pub fn errors(&self) -> &[(String, ValidationError)] {
        &self.errors
    }
}

impl Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, (field, error)) in self.errors.iter().enumerate() {
            if index > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{field}: {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_get_error() {
        let mut errors = ValidationErrors::new();
        errors.push("email", ValidationError::new("is required"));

        assert_eq!(errors.len(), 1);
        assert_eq!(errors.get("email").map(ValidationError::message), Some("is required"));
        assert!(errors.get("missing").is_none());
    }

    #[test]
    fn display_lists_field_and_message() {
        let mut errors = ValidationErrors::new();
        errors.push("email", "is required");
        errors.push("age", "must be at least 18");

        assert_eq!(errors.to_string(), "email: is required; age: must be at least 18");
    }
}
