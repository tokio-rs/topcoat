use std::ops::Deref;

use crate::{Surrogate, Surrogated};

impl Surrogated for () {
    type Surrogate = ();
    fn into_surrogate(self) -> Self::Surrogate {}
}

impl Surrogate for () {
    type Real = ();
    fn into_real(self) -> Self::Real {}
}

/// A tuple whose elements can be borrowed as runtime values.
pub trait TupleProject<'a> {
    /// A tuple of the borrowed elements' surrogates.
    type Projected;

    fn project(&'a self) -> Self::Projected;
}

/// A borrowed tuple of runtime values.
///
/// Field access reads the borrowed elements, so `.0` on a borrowed pair
/// borrows its first element.
pub struct TupleRefSurrogate<'a, T>
where
    T: TupleProject<'a>,
{
    real: &'a T,
    projected: T::Projected,
}

impl<'a, T> TupleRefSurrogate<'a, T>
where
    T: TupleProject<'a>,
{
    fn new(real: &'a T) -> Self {
        Self {
            real,
            projected: real.project(),
        }
    }
}

impl<'a, T> TupleRefSurrogate<'a, T>
where
    T: TupleProject<'a> + Surrogated + Clone,
{
    /// Clones the borrowed tuple into an owned tuple.
    // Cloning a `&(A, B)` yields an `(A, B)`, which `Clone` cannot express.
    #[must_use]
    #[allow(clippy::should_implement_trait)]
    pub fn clone(&self) -> T::Surrogate {
        self.real.clone().into_surrogate()
    }
}

impl<'a, T> Deref for TupleRefSurrogate<'a, T>
where
    T: TupleProject<'a>,
{
    type Target = T::Projected;

    fn deref(&self) -> &Self::Target {
        &self.projected
    }
}

impl<'a, T> serde::Serialize for TupleRefSurrogate<'a, T>
where
    T: TupleProject<'a>,
    T::Projected: serde::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.projected.serialize(serializer)
    }
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

        impl<'a, $($t: 'a),+> TupleProject<'a> for ($($t,)+)
        where
            $(&'a $t: Surrogated,)+
        {
            type Projected = ($(<&'a $t as Surrogated>::Surrogate,)+);

            fn project(&'a self) -> Self::Projected {
                ($((&self.$idx).into_surrogate(),)+)
            }
        }

        impl<'a, $($t: 'a),+> Surrogated for &'a ($($t,)+)
        where
            $(&'a $t: Surrogated,)+
        {
            type Surrogate = TupleRefSurrogate<'a, ($($t,)+)>;

            fn into_surrogate(self) -> Self::Surrogate {
                TupleRefSurrogate::new(self)
            }
        }

        impl<'a, $($t: 'a),+> Surrogate for TupleRefSurrogate<'a, ($($t,)+)>
        where
            $(&'a $t: Surrogated,)+
        {
            type Real = &'a ($($t,)+);

            fn into_real(self) -> Self::Real {
                self.real
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
