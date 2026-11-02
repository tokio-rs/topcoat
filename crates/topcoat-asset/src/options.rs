use std::borrow::Cow;

use topcoat_core::fnv1a::Fnv1a;

use crate::{ConstReader, ConstWriter};

/// Options that control how an asset is bundled.
///
/// Set these fields with the options of `asset!`, such as
/// `asset!("./logo.png", rename: "logo")`.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetOptions {
    /// Replaces the file stem (everything before the final `.`) in the bundled
    /// output. The extension and content hash are kept. An empty string drops the
    /// stem, leaving just the hash and extension as the filename.
    pub rename: Option<Cow<'static, str>>,
    /// Overrides the output extension, without the leading dot. Useful when the
    /// source has no extension or the wrong one. An empty string drops the
    /// extension.
    pub extension: Option<Cow<'static, str>>,
    /// The expected hash of the raw, unbundled source file, written as
    /// `"sha256:<hex>"`. Bundling fails if the file's hash differs. Recommended
    /// for remote assets.
    pub checksum: Option<Cow<'static, str>>,
    /// Overrides the `Content-Type` the asset is served with. When unset, it is
    /// guessed from the bundled file's extension.
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

    pub(crate) const fn encoded_len(&self) -> usize {
        str_opt_len(cow_as_str(self.rename.as_ref()))
            + str_opt_len(cow_as_str(self.extension.as_ref()))
            + str_opt_len(cow_as_str(self.checksum.as_ref()))
            + str_opt_len(cow_as_str(self.content_type.as_ref()))
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

/// The number of bytes [`ConstWriter::write_str_opt`] writes for `s`.
const fn str_opt_len(s: Option<&str>) -> usize {
    match s {
        None => 1,
        Some(s) => 1 + 2 + s.len(),
    }
}
