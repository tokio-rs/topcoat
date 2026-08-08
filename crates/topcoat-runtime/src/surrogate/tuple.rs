use crate::{Surrogate, Surrogated};

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

#[cfg(test)]
mod tests {
    use crate::Surrogated;

    /// A captured value reaches the browser as `cx.hydrate(<json>)`, and the
    /// browser reads a tuple as an array because `expr!` compiles tuple field
    /// access to array indexing. Pin the shape both sides depend on.
    #[test]
    fn a_tuple_surrogate_serializes_as_an_array() {
        let pair = (1.5f64, 2.5f64).into_surrogate();
        assert_eq!(serde_json::to_string(&pair).unwrap(), "[1.5,2.5]");
    }

    #[test]
    fn the_elements_of_a_tuple_keep_their_own_encoding() {
        let mixed = (1.0f64, Some(2.0f64), true).into_surrogate();
        assert_eq!(
            serde_json::to_string(&mixed).unwrap(),
            r#"[1.0,{"t":"Option","v":2.0},true]"#
        );
    }

    #[test]
    fn a_nested_tuple_nests_its_arrays() {
        let nested = ((1.0f64, 2.0f64), 3.0f64).into_surrogate();
        assert_eq!(serde_json::to_string(&nested).unwrap(), "[[1.0,2.0],3.0]");
    }
}
