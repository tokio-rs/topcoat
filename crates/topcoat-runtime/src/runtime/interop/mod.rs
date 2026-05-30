mod _f64;
mod signal;

pub use _f64::*;
pub use signal::*;

pub trait Interop {
    type Surrogate;

    fn to_js(&self, out: &mut String);
    fn into_surrogate(self) -> Self::Surrogate;
    fn to_surrogate_ref(&self) -> &Self::Surrogate;
}
