use crate::runtime::{Surrogate, Surrogated};

impl Surrogated for () {
    type Surrogate = ();
    fn into_surrogate(self) -> Self::Surrogate {}
}

impl Surrogate for () {
    type Real = ();
    fn into_real(self) -> Self::Real {}
}

macro_rules! impl_tuple_surrogate {
    ($($t:ident $idx:tt),+ $(,)?) => {
        impl<$($t),+> Surrogated for ($($t,)+)
        where
            $($t: Surrogated,)+
        {
            type Surrogate = ($(<$t as Surrogated>::Surrogate,)+);

            fn into_surrogate(self) -> Self::Surrogate {
                ($(self.$idx.into_surrogate(),)+)
            }
        }

        impl<$($t),+> Surrogate for ($($t,)+)
        where
            $($t: Surrogate,)+
        {
            type Real = ($(<$t as Surrogate>::Real,)+);

            fn into_real(self) -> Self::Real {
                ($(self.$idx.into_real(),)+)
            }
        }
    };
}

impl_tuple_surrogate!(T1 0);
impl_tuple_surrogate!(T1 0, T2 1);
impl_tuple_surrogate!(T1 0, T2 1, T3 2);
impl_tuple_surrogate!(T1 0, T2 1, T3 2, T4 3);
impl_tuple_surrogate!(T1 0, T2 1, T3 2, T4 3, T5 4);
impl_tuple_surrogate!(T1 0, T2 1, T3 2, T4 3, T5 4, T6 5);
impl_tuple_surrogate!(T1 0, T2 1, T3 2, T4 3, T5 4, T6 5, T7 6);
impl_tuple_surrogate!(T1 0, T2 1, T3 2, T4 3, T5 4, T6 5, T7 6, T8 7);
impl_tuple_surrogate!(T1 0, T2 1, T3 2, T4 3, T5 4, T6 5, T7 6, T8 7, T9 8);
impl_tuple_surrogate!(T1 0, T2 1, T3 2, T4 3, T5 4, T6 5, T7 6, T8 7, T9 8, T10 9);
impl_tuple_surrogate!(T1 0, T2 1, T3 2, T4 3, T5 4, T6 5, T7 6, T8 7, T9 8, T10 9, T11 10);
impl_tuple_surrogate!(
    T1 0, T2 1, T3 2, T4 3, T5 4, T6 5, T7 6, T8 7, T9 8, T10 9, T11 10, T12 11,
);
