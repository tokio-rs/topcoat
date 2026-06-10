use ref_cast::RefCast;
use topcoat_runtime::runtime::Surrogated;

#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Action<A, R>(crate::runtime::Action<A, R>);

impl<A, R> Action<A, R> {
    #[inline]
    pub(crate) const fn new(v: crate::runtime::Action<A, R>) -> Self {
        Self(v)
    }
}

impl<A, R> Action<A, R>
where
    A: Surrogated,
{
    pub fn call(self, _args: A::Surrogate) -> R {
        panic!("actions cannot be executed on the server");
    }
}

topcoat_runtime::impl_surrogate!({A, R} crate::runtime::Action<A, R>, Action<A, R>);
topcoat_runtime::impl_surrogate_ref!({A, R} crate::runtime::Action<A, R>, Action<A, R>);
topcoat_runtime::impl_surrogate_mut!({A, R} crate::runtime::Action<A, R>, Action<A, R>);
