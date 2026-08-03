use std::any::{Any, TypeId};

use super::Cx;

/// A tuple of values that can be installed by [`Cx::with_values`].
///
/// This trait is implemented for tuples containing two through twelve values.
/// It is sealed and cannot be implemented outside Topcoat.
pub trait ContextValues: Sealed {}

impl<T> ContextValues for T where T: Sealed {}

pub trait Sealed {
    fn install(self, cx: &mut Cx);
}

macro_rules! impl_context_values {
    (@one $(($type:ident, $index:tt)),+) => {
        impl<$($type),+> Sealed for ($($type,)+)
        where
            $($type: Any + Send + Sync,)+
        {
            fn install(self, cx: &mut Cx) {
                let types = [$(TypeId::of::<$type>()),+];
                assert!(
                    types
                        .iter()
                        .enumerate()
                        .all(|(index, type_id)| !types[..index].contains(type_id)),
                    "a context scope cannot contain duplicate value types"
                );
                $(cx.install_scoped_value(self.$index);)+
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
