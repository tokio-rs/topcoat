use std::fmt::{Display, Write};

use serde::Serialize;
use topcoat_core::{
    base_url::base_url,
    context::Cx,
    url_form::{UrlForm, url_form},
};

use crate::Path;

pub trait HrefTarget {
    fn path<'cx>(&self, cx: &'cx Cx) -> &'cx Path;
}

impl HrefTarget for &'static Path {
    fn path<'cx>(&self, _cx: &'cx Cx) -> &'cx Path {
        self
    }
}

impl HrefTarget for &'static str {
    fn path<'cx>(&self, cx: &'cx Cx) -> &'cx Path {
        HrefTarget::path(&Path::new(self), cx)
    }
}

pub trait HrefParam {
    type Value: Display + ?Sized;

    fn name(&self) -> &str;
    fn value(&self) -> &Self::Value;
}

impl<T> HrefParam for &T
where
    T: HrefParam,
{
    type Value = T::Value;

    fn name(&self) -> &str {
        (*self).name()
    }

    fn value(&self) -> &Self::Value {
        (*self).value()
    }
}

pub trait HrefParams {
    fn assign(&self, path: &Path, out: &mut String);
}

pub fn href<T, P>(target: T, params: P) -> Href<T, P, ()>
where
    T: HrefTarget,
    P: HrefParams,
{
    Href {
        target,
        params,
        query: (),
        fragment: String::new(),
    }
}

pub struct Href<T, P, Q, F> {
    target: T,
    params: P,
    query: Q,
    fragment: Option<F>,
}

impl<T, P, Q, F> Href<T, P, Q, F>
where
    T: HrefTarget,
    P: HrefParams,
    Q: Serialize,
    F: Display,
{
    pub fn resolve(self, cx: &Cx) -> String {
        let mut buf = String::new();
        match url_form(cx) {
            UrlForm::Absolute => buf += base_url(cx).as_str(),
            UrlForm::Relative => {}
        }

        self.params.assign(self.target.path(cx), &mut buf);

        if let Some(fragment) = self.fragment {
            write!(buf, "#{fragment}");
        }

        self.buf
    }
}
