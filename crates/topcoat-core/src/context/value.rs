use std::any::{Any, TypeId};

use super::binding::{BindingSet, Registry};

/// A tuple of values that can be installed by [`super::Cx::with_values`].
///
/// This trait is implemented for tuples containing two through twelve values.
/// It is sealed and cannot be implemented outside Topcoat.
pub trait ContextValues: Sealed {}

impl<T> ContextValues for T where T: Sealed {}

pub struct Installer<'a> {
    bindings: &'a mut BindingSet,
    registry: &'a mut Registry,
}

impl<'a> Installer<'a> {
    pub(super) fn new(bindings: &'a mut BindingSet, registry: &'a mut Registry) -> Self {
        Self { bindings, registry }
    }

    fn install<T>(&mut self, value: T)
    where
        T: Any + Send + Sync,
    {
        self.bindings.install_scoped(self.registry, value);
    }
}

pub trait Sealed {
    fn assert_unique();

    fn install(self, installer: &mut Installer<'_>);
}

macro_rules! impl_context_values {
    (@one $(($type:ident, $index:tt)),+) => {
        impl<$($type),+> Sealed for ($($type,)+)
        where
            $($type: Any + Send + Sync,)+
        {
            fn assert_unique() {
                let types = [$(TypeId::of::<$type>()),+];
                assert!(
                    types
                        .iter()
                        .enumerate()
                        .all(|(index, type_id)| !types[..index].contains(type_id)),
                    "a context scope cannot contain duplicate value types"
                );
            }

            fn install(self, installer: &mut Installer<'_>) {
                $(installer.install(self.$index);)+
            }
        }
    };
    (@each [$(($type:ident, $index:tt)),+] ($next:ident, $next_index:tt) $(, $rest:tt)*) => {
        impl_context_values!(@one $(($type, $index)),+);
        impl_context_values!(@each [$(($type, $index)),+, ($next, $next_index)] $($rest),*);
    };
    (@each [$(($type:ident, $index:tt)),+]) => {
        impl_context_values!(@one $(($type, $index)),+);
    };
}

impl_context_values!(@each [(T1, 0), (T2, 1)]
    (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6),
    (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11)
);
