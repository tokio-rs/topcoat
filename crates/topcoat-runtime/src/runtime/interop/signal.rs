use ref_cast::{RefCastCustom, ref_cast_custom};

use crate::runtime::{Interop, Signal};

#[derive(RefCastCustom, Clone, Copy)]
#[repr(transparent)]
pub struct WriteSignal<'a, T>(Signal<'a, T>);

impl<'a, T> WriteSignal<'a, T> {
    #[ref_cast_custom]
    pub(crate) const fn from_ref<'b>(v: &'b Signal<'a, T>) -> &'b Self;
}

impl<'a, T> WriteSignal<'a, T>
where
    T: Interop,
{
    pub fn read(&self) -> &'a T::Surrogate {
        self.0.read().to_surrogate_ref()
    }

    pub fn set(&self, _v: T::Surrogate) {
        panic!("signals cannot be written to inside of a server-side expression");
    }
}

impl<'a, T> Interop for Signal<'a, T> {
    type Surrogate = WriteSignal<'a, T>;

    fn to_js(&self, out: &mut String) {
        *out += "__context.signal(";
        *out += &serde_json::to_string(&self.id()).unwrap();
        *out += ")";
    }

    fn into_surrogate(self) -> Self::Surrogate {
        WriteSignal(self)
    }

    fn to_surrogate_ref(&self) -> &Self::Surrogate {
        WriteSignal::from_ref(self)
    }
}
