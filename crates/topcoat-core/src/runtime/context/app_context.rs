use std::any::{Any, type_name};
use std::sync::Arc;

use super::ContextMap;

/// Application services shared by request and non-request render contexts.
///
/// An `AppContext` is immutable after construction and cheap to clone. Extend
/// an existing context with [`AppContext::builder_from`] when another runtime
/// needs the same services plus additional values.
#[derive(Clone, Debug, Default)]
pub struct AppContext {
    layers: Arc<[Arc<ContextMap>]>,
}

impl AppContext {
    /// Starts building an empty application context.
    #[must_use]
    pub fn builder() -> AppContextBuilder {
        AppContextBuilder::default()
    }

    /// Starts building an application context that includes `base`.
    #[must_use]
    pub fn builder_from(base: AppContext) -> AppContextBuilder {
        AppContextBuilder {
            base,
            context: ContextMap::new(),
        }
    }

    /// Returns the registered value of type `T`, or `None` when it is absent.
    #[must_use]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.layers.iter().rev().find_map(|layer| layer.get::<T>())
    }

    /// Returns the registered value of type `T`.
    ///
    /// # Panics
    ///
    /// Panics when no value of type `T` is registered.
    #[must_use]
    pub fn require<T>(&self) -> &T
    where
        T: Any + Send + Sync,
    {
        self.get::<T>().unwrap_or_else(|| {
            panic!(
                "attempted to access app context of type `{:?}`, but this type was not registered for this context",
                type_name::<T>()
            )
        })
    }

    /// Returns `true` when a value of type `T` is registered.
    #[must_use]
    pub fn contains<T>(&self) -> bool
    where
        T: Any + Send + Sync,
    {
        self.get::<T>().is_some()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn len(&self) -> usize {
        self.layers.iter().map(|layer| layer.len()).sum()
    }

    #[doc(hidden)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<Arc<ContextMap>> for AppContext {
    fn from(context: Arc<ContextMap>) -> Self {
        Self {
            layers: Arc::from([context]),
        }
    }
}

/// Builds an immutable [`AppContext`].
#[derive(Debug, Default)]
pub struct AppContextBuilder {
    base: AppContext,
    context: ContextMap,
}

impl AppContextBuilder {
    /// Registers a value under its concrete type.
    ///
    /// # Panics
    ///
    /// Panics when this builder or its base context already contains `T`.
    #[must_use]
    pub fn insert<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        assert!(
            !self.base.contains::<T>() && self.context.insert(value).is_none(),
            "duplicate context entry for type `{:?}`",
            type_name::<T>()
        );
        self
    }

    /// Returns a value registered directly on this builder or its base.
    #[must_use]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.context.get::<T>().or_else(|| self.base.get::<T>())
    }

    /// Returns a mutable value registered directly on this builder.
    ///
    /// Values inherited from the immutable base context cannot be mutated.
    #[must_use]
    pub fn get_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any + Send + Sync,
    {
        self.context.get_mut::<T>()
    }

    /// Builds the immutable application context.
    #[must_use]
    pub fn build(self) -> AppContext {
        let mut layers = self.base.layers.to_vec();
        layers.push(Arc::new(self.context));
        AppContext {
            layers: layers.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Database(&'static str);

    #[derive(Debug, PartialEq)]
    struct Config(u32);

    #[test]
    fn contexts_are_cloneable_and_extendable() {
        let services = AppContext::builder().insert(Database("primary")).build();
        let extended = AppContext::builder_from(services.clone())
            .insert(Config(42))
            .build();

        assert_eq!(services.get::<Database>(), Some(&Database("primary")));
        assert_eq!(services.get::<Config>(), None);
        assert_eq!(extended.get::<Database>(), Some(&Database("primary")));
        assert_eq!(extended.get::<Config>(), Some(&Config(42)));
    }

    #[test]
    #[should_panic(expected = "duplicate context entry")]
    fn extending_rejects_duplicate_types() {
        let services = AppContext::builder().insert(Database("primary")).build();
        let _ = AppContext::builder_from(services).insert(Database("replica"));
    }
}
