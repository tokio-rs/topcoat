use ref_cast::{RefCastCustom, ref_cast_custom};
use serde::Serialize;

use crate::{JsCallable, Value};
use std::primitive::str as StdStr;

#[derive(RefCastCustom, Serialize)]
#[repr(transparent)]
#[serde(transparent)]
#[allow(non_camel_case_types)]
pub struct str {
    inner: StdStr,
}

impl str {
    #[ref_cast_custom]
    pub(crate) fn ref_cast(s: &StdStr) -> &Self;

    #[allow(clippy::should_implement_trait)]
    pub fn to_owned(&self) -> crate::string::String {
        crate::string::String::new(self.inner.to_owned())
    }
}

impl JsCallable for &str {
    fn js_call(method: &StdStr, _out: &mut String) {
        match method {
            // str and String are the same thing in JS, to_owned is not necessary.
            "to_owned" => {}
            "to_string" => {}
            _ => unreachable!(),
        }
    }
}

impl Value for StdStr {
    type Surrogate = str;

    fn ref_cast(&self) -> &Self::Surrogate {
        Self::Surrogate::ref_cast(self)
    }
}
