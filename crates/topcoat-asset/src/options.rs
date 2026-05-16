use std::borrow::Cow;

use crate::cursor::{ConstReader, ConstWriter};

#[derive(Debug, Clone, PartialEq)]
pub struct AssetOptions {
    pub rename: Option<Cow<'static, str>>,
    pub extension: Option<Cow<'static, str>>,
    pub hash: Option<Cow<'static, str>>,
}

impl AssetOptions {
    pub const NONE: Self = Self {
        rename: None,
        extension: None,
        hash: None,
    };

    pub fn rename(&self) -> Option<&str> {
        self.rename.as_deref()
    }

    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }

    pub fn hash(&self) -> Option<&str> {
        self.hash.as_deref()
    }

    pub(crate) const fn encode_into(&self, w: &mut ConstWriter<'_>) {
        w.write_str_opt(cow_as_str(&self.rename));
        w.write_str_opt(cow_as_str(&self.extension));
        w.write_str_opt(cow_as_str(&self.hash));
    }

    pub(crate) fn decode_from(r: &mut ConstReader<'_>) -> Option<Self> {
        Some(Self {
            rename: r.read_str_opt()?.map(|s| Cow::Owned(s.to_owned())),
            extension: r.read_str_opt()?.map(|s| Cow::Owned(s.to_owned())),
            hash: r.read_str_opt()?.map(|s| Cow::Owned(s.to_owned())),
        })
    }
}

const fn cow_as_str<'a>(c: &'a Option<Cow<'static, str>>) -> Option<&'a str> {
    match c {
        None => None,
        Some(Cow::Borrowed(s)) => Some(s),
        Some(Cow::Owned(s)) => Some(s.as_str()),
    }
}

#[macro_export]
macro_rules! asset_options {
    ($($field:ident $(: $expr:expr)?),*) => {
        $crate::AssetOptions {
            $($field: ::core::option::Option::Some(::std::borrow::Cow::Borrowed($($expr)?)),)*
            ..$crate::AssetOptions::NONE
        }
    };
}
