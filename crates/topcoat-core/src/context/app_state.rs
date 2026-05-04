use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::context::Cx;

pub fn app_state<T>(cx: &Cx) -> &T
where
    T: Any + Send + Sync,
{
    cx.state.get::<T>()
}

#[derive(Default, Debug)]
pub struct AppState {
    entries: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<T>(&mut self, value: T)
    where
        T: Any + Send + Sync,
    {
        if self
            .entries
            .insert(TypeId::of::<T>(), Box::new(value))
            .is_some()
        {
            panic!("duplicate state entry for type `{:?}`", TypeId::of::<T>())
        }
    }

    fn get<T>(&self) -> &T
    where
        T: Any + Send + Sync,
    {
        match self.entries.get(&TypeId::of::<T>()).as_ref() {
            Some(&value) => value.downcast_ref().unwrap(),
            None => {
                panic!(
                    "attempted to access app state of type `{:?}`, but this type was not registered",
                    TypeId::of::<T>()
                );
            }
        }
    }
}
