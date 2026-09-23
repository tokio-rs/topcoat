use std::borrow::Cow;

use topcoat_core::fnv1a::Fnv1a;

use crate::{ConstReader, ConstWriter};

/// Options that control how an asset is bundled and served.
///
/// Set them through the named arguments of [`asset!`](crate::asset), or
/// build a value with [`asset_options!`](crate::asset_options). Every field
/// is optional and defaults to `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetOptions {
    /// Replaces the file stem (everything before the last `.`) of the
    /// bundled file. The hash and the extension are kept. An empty string
    /// removes the stem, so the filename is just the hash and the extension.
    pub rename: Option<Cow<'static, str>>,
    /// Replaces the extension of the bundled file, without the leading dot.
    /// Useful when the source has no extension or the wrong one. An empty
    /// string removes the extension.
    pub extension: Option<Cow<'static, str>>,
    /// The expected hash of the source file, as `algorithm:hex`. The only
    /// supported algorithm is `sha256`, for example `"sha256:e3b0c442..."`.
    ///
    /// The bundler fails with
    /// [`AssetError::ChecksumMismatch`](crate::AssetError::ChecksumMismatch)
    /// if the file has a different hash, and with
    /// [`AssetError::UnsupportedChecksum`](crate::AssetError::UnsupportedChecksum)
    /// if the value does not start with `sha256:`. Recommended for remote
    /// assets, so that a changed remote file fails the build.
    pub checksum: Option<Cow<'static, str>>,
    /// The `Content-Type` the asset is served with. When unset, the bundler
    /// guesses it from the extension of the bundled file.
    pub content_type: Option<Cow<'static, str>>,
}

impl AssetOptions {
    /// Options with every field set to `None`.
    pub const NONE: Self = Self {
        rename: None,
        extension: None,
        checksum: None,
        content_type: None,
    };

    /// Returns the [`rename`](Self::rename) option, if set.
    #[must_use]
    pub fn rename(&self) -> Option<&str> {
        self.rename.as_deref()
    }

    /// Returns the [`extension`](Self::extension) option, if set.
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }

    /// Returns the [`checksum`](Self::checksum) option, if set.
    #[must_use]
    pub fn checksum(&self) -> Option<&str> {
        self.checksum.as_deref()
    }

    /// Returns the [`content_type`](Self::content_type) option, if set.
    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    pub(crate) const fn encode_into(&self, w: &mut ConstWriter<'_>) {
        w.write_str_opt(cow_as_str(self.rename.as_ref()));
        w.write_str_opt(cow_as_str(self.extension.as_ref()));
        w.write_str_opt(cow_as_str(self.checksum.as_ref()));
        w.write_str_opt(cow_as_str(self.content_type.as_ref()));
    }

    pub(crate) const fn hash_into(&self, mut h: Fnv1a<u64>) -> Fnv1a<u64> {
        h = hash_opt_str(h, cow_as_str(self.rename.as_ref()));
        h = hash_opt_str(h, cow_as_str(self.extension.as_ref()));
        h = hash_opt_str(h, cow_as_str(self.checksum.as_ref()));
        h = hash_opt_str(h, cow_as_str(self.content_type.as_ref()));
        h
    }

    pub(crate) fn decode_from(r: &mut ConstReader<'_>) -> Option<Self> {
        Some(Self {
            rename: r.read_str_opt()?.map(|s| Cow::Owned(s.to_owned())),
            extension: r.read_str_opt()?.map(|s| Cow::Owned(s.to_owned())),
            checksum: r.read_str_opt()?.map(|s| Cow::Owned(s.to_owned())),
            content_type: r.read_str_opt()?.map(|s| Cow::Owned(s.to_owned())),
        })
    }
}

const fn cow_as_str<'a>(c: Option<&'a Cow<'static, str>>) -> Option<&'a str> {
    match c {
        None => None,
        Some(Cow::Borrowed(s)) => Some(s),
        Some(Cow::Owned(s)) => Some(s.as_str()),
    }
}

const fn hash_opt_str(h: Fnv1a<u64>, s: Option<&str>) -> Fnv1a<u64> {
    match s {
        None => h.write(&[0]),
        Some(s) => h.write(&[1]).write(s.as_bytes()),
    }
}

/// Builds an [`AssetOptions`] from a comma-separated list of fields.
///
/// Write each field as `name: value`, where the value is a `&'static str`,
/// usually a string literal. Fields you leave out stay `None`. The macro can
/// be used to initialize a `const`.
///
/// ```rust
/// use topcoat_asset::{AssetOptions, asset_options};
///
/// const OPTS: AssetOptions = asset_options!(rename: "primary", extension: "woff2");
/// ```
#[macro_export]
macro_rules! asset_options {
    ($($field:ident $(: $expr:expr)?),* $(,)?) => {{
        #[allow(clippy::needless_update)]
        $crate::AssetOptions {
            $($field: ::core::option::Option::Some(::std::borrow::Cow::Borrowed($($expr)?)),)*
            ..$crate::AssetOptions::NONE
        }
    }};
}
