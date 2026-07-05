use std::borrow::Cow;

use topcoat_core::runtime::fnv1a;

use crate::{ConstReader, ConstWriter};

/// Options that control how an asset is bundled.
///
/// Usually set via the [`asset!`](crate::asset) or
/// [`asset_options!`](crate::asset_options) macros rather than
/// constructed directly. See the [`asset!`](crate::asset) docs for what
/// each field does.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetOptions {
    /// Replace the file stem (everything before the final `.`) in the
    /// bundled output. The extension and content-hash suffix are kept.
    /// An empty string drops the stem entirely, leaving just the hash
    /// (and extension, if any) as the filename.
    pub rename: Option<Cow<'static, str>>,
    /// Override the output extension (without the leading dot). Useful
    /// when the source has no extension or the wrong one. An empty
    /// string drops the extension entirely.
    pub extension: Option<Cow<'static, str>>,
    /// Expected hash of the raw, unbundled source file, as an
    /// `algorithm:hex` string. Only `sha256` is currently supported, e.g.
    /// `"sha256:e3b0c442..."`. The bundler fails with
    /// [`AssetError::ChecksumMismatch`](crate::AssetError) if the source's
    /// actual hash differs, or
    /// [`AssetError::UnsupportedChecksum`](crate::AssetError) if the
    /// algorithm prefix is missing or unsupported. Recommended for remote
    /// assets.
    pub checksum: Option<Cow<'static, str>>,
    /// Override the `Content-Type` the asset is served with. When unset, the
    /// bundler guesses it from the bundled file's extension.
    pub content_type: Option<Cow<'static, str>>,
}

impl AssetOptions {
    /// All options unset.
    pub const NONE: Self = Self {
        rename: None,
        extension: None,
        checksum: None,
        content_type: None,
    };

    /// Returns the configured [`rename`](Self::rename) value, if any.
    #[must_use]
    pub fn rename(&self) -> Option<&str> {
        self.rename.as_deref()
    }

    /// Returns the configured [`extension`](Self::extension) value, if any.
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }

    /// Returns the configured [`checksum`](Self::checksum) value, if any.
    #[must_use]
    pub fn checksum(&self) -> Option<&str> {
        self.checksum.as_deref()
    }

    /// Returns the configured [`content_type`](Self::content_type) value, if any.
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

    pub(crate) const fn hash_into(&self, mut h: u64) -> u64 {
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

const fn hash_opt_str(h: u64, s: Option<&str>) -> u64 {
    match s {
        None => fnv1a::hash_continue(h, &[0]),
        Some(s) => {
            let h = fnv1a::hash_continue(h, &[1]);
            fnv1a::hash_continue(h, s.as_bytes())
        }
    }
}

/// Build an [`AssetOptions`] from a comma-separated list of fields.
///
/// Each field is either `name: "literal"` to set that option, or a bare
/// `name` (which expects a const string in scope of the same name).
/// Omitted fields stay `None`.
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
