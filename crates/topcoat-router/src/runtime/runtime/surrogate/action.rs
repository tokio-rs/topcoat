use ref_cast::RefCast;
use topcoat_runtime::runtime::Surrogated;

#[derive(Debug, RefCast)]
pub struct Action<F>(crate::runtime::Action<F>);

topcoat_runtime::impl_surrogate!({F} crate::runtime::Action<F>, Action<F>);
