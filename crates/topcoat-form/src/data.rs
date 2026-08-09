use std::{collections::HashMap, hash::BuildHasher};

use crate::Value;

/// A type that can supply field values for a validation schema.
///
/// Implement this for the struct or map that holds the data you want to
/// validate. The schema asks for each field by name and runs its validators
/// on the returned [`Value`].
pub trait ValidationData: Send + Sync {
    /// Returns the value of the field named `name`, or [`None`] when the
    /// implementor does not expose that field.
    ///
    /// Return [`Value::Missing`] when the field is expected but absent, so
    /// validators such as [`required`](crate::Field::required) can report it.
    fn field(&self, name: &str) -> Option<Value>;
}

impl<S: BuildHasher + Send + Sync> ValidationData for HashMap<String, String, S> {
    fn field(&self, name: &str) -> Option<Value> {
        self.get(name).cloned().map(Value::String)
    }
}

impl<S: BuildHasher + Send + Sync> ValidationData for HashMap<String, Value, S> {
    fn field(&self, name: &str) -> Option<Value> {
        self.get(name).cloned()
    }
}

impl ValidationData for Vec<(String, String)> {
    fn field(&self, name: &str) -> Option<Value> {
        self.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| Value::String(value.clone()))
    }
}

impl ValidationData for [(String, String)] {
    fn field(&self, name: &str) -> Option<Value> {
        self.iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| Value::String(value.clone()))
    }
}

impl<T: ValidationData + ?Sized> ValidationData for &T {
    fn field(&self, name: &str) -> Option<Value> {
        (**self).field(name)
    }
}

impl<T: ValidationData + ?Sized> ValidationData for Box<T> {
    fn field(&self, name: &str) -> Option<Value> {
        (**self).field(name)
    }
}
