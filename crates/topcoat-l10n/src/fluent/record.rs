use topcoat_core::cursor::{ConstReader, ConstWriter};

/// A `t!` call, recovered from a compiled binary.
///
/// `t!` embeds a record of itself in the binary, which is what the created
/// [`Translation`](crate::Translation) reads its key from. Tools find the
/// records of every `t!` call that the compiler kept with
/// [`find_in_binary`](Self::find_in_binary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTranslation<'a> {
    /// The ID of the namespace of the `t!`.
    pub namespace: &'a str,
    /// The ID of the message.
    pub id: &'a str,
    /// The attribute of the message, if any.
    pub attribute: Option<&'a str>,
    /// The names of the arguments, as written.
    pub arguments: Vec<&'a str>,
    /// The source file of the call, as given by `file!()`.
    pub file: &'a str,
    /// The line of the key, starting at 1.
    pub line: u32,
    /// The column of the key, starting at 1.
    pub column: u32,
}

impl<'a> RawTranslation<'a> {
    /// Returns the size of the record that [`encode`](Self::encode) creates.
    #[doc(hidden)]
    #[must_use]
    pub const fn encoded_len(
        namespace: &str,
        id: &str,
        attribute: Option<&str>,
        arguments: &[&str],
        file: &str,
    ) -> usize {
        let mut len = PREFIX_LEN
            + str_len(namespace)
            + str_len(id)
            + str_opt_len(attribute)
            + 2
            + str_len(file)
            + 8;
        let mut i = 0;
        while i < arguments.len() {
            len += str_len(arguments[i]);
            i += 1;
        }
        len
    }

    /// Creates the record of a `t!` call. `N` must be the
    /// [`encoded_len`](Self::encoded_len) of the same values.
    #[doc(hidden)]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn encode<const N: usize>(
        namespace: &str,
        id: &str,
        attribute: Option<&str>,
        arguments: &[&str],
        file: &str,
        line: u32,
        column: u32,
    ) -> [u8; N] {
        let mut out = [0; N];
        let mut w = ConstWriter::new(&mut out);
        w.write_bytes(&prefix(TRANSLATION));
        w.write_str(namespace);
        w.write_str(id);
        w.write_str_opt(attribute);
        w.write_u16_le(arguments.len() as u16);
        let mut i = 0;
        while i < arguments.len() {
            w.write_str(arguments[i]);
            i += 1;
        }
        w.write_str(file);
        w.write_u32_le(line);
        w.write_u32_le(column);
        out
    }

    /// Reads the record at the start of `bytes`, or returns `None` when it is
    /// not one.
    #[must_use]
    pub fn decode(bytes: &'a [u8]) -> Option<Self> {
        let mut r = ConstReader::new(bytes);
        let (namespace, id, attribute) = Self::decode_key(&mut r)?;
        let count = r.read_u16_le()?;
        let arguments = (0..count).map(|_| r.read_str()).collect::<Option<_>>()?;
        Some(Self {
            namespace,
            id,
            attribute,
            arguments,
            file: r.read_str()?,
            line: r.read_u32_le()?,
            column: r.read_u32_le()?,
        })
    }

    /// Reads the namespace, the message ID and the attribute of the record.
    pub(crate) fn decode_key(
        r: &mut ConstReader<'a>,
    ) -> Option<(&'a str, &'a str, Option<&'a str>)> {
        if r.read_bytes(PREFIX_LEN)? != prefix(TRANSLATION) {
            return None;
        }
        Some((r.read_str()?, r.read_str()?, r.read_str_opt()?))
    }

    /// Returns the records of every `t!` call in a compiled binary.
    #[cfg(feature = "check")]
    #[must_use]
    pub fn find_in_binary(binary: &'a [u8]) -> Vec<Self> {
        find_in_binary(binary, TRANSLATION, Self::decode)
    }
}

/// A `messages!` invocation, recovered from a compiled binary.
///
/// `messages!` embeds a record of itself in the binary. Tools find the records
/// of every namespace with [`find_in_binary`](Self::find_in_binary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawNamespace<'a> {
    /// The ID of the namespace.
    pub id: &'a str,
    /// The absolute path of the source file that invokes `messages!`.
    pub source_file: &'a str,
    /// The line of the invocation, starting at 1.
    pub line: u32,
    /// The path of the Fluent files, as written.
    pub pattern: &'a str,
    /// The locales, with the files that were embedded for each.
    pub locales: Vec<RawNamespaceLocale<'a>>,
}

/// The files that were embedded for one locale of a [`RawNamespace`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawNamespaceLocale<'a> {
    /// The locale, as matched by the path.
    pub locale: &'a str,
    /// The absolute path of each file.
    pub files: Vec<&'a str>,
}

impl<'a> RawNamespace<'a> {
    /// Returns the size of the record that [`encode`](Self::encode) creates.
    #[doc(hidden)]
    #[must_use]
    pub const fn encoded_len(
        id: &str,
        source_file: &str,
        pattern: &str,
        locales: &[(&str, &[&str])],
    ) -> usize {
        let mut len = PREFIX_LEN + str_len(id) + str_len(source_file) + 4 + str_len(pattern) + 2;
        let mut i = 0;
        while i < locales.len() {
            let (locale, files) = locales[i];
            len += str_len(locale) + 2;
            let mut j = 0;
            while j < files.len() {
                len += str_len(files[j]);
                j += 1;
            }
            i += 1;
        }
        len
    }

    /// Creates the record of a `messages!` invocation. `N` must be the
    /// [`encoded_len`](Self::encoded_len) of the same values.
    #[doc(hidden)]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn encode<const N: usize>(
        id: &str,
        source_file: &str,
        line: u32,
        pattern: &str,
        locales: &[(&str, &[&str])],
    ) -> [u8; N] {
        let mut out = [0; N];
        let mut w = ConstWriter::new(&mut out);
        w.write_bytes(&prefix(NAMESPACE));
        w.write_str(id);
        w.write_str(source_file);
        w.write_u32_le(line);
        w.write_str(pattern);
        w.write_u16_le(locales.len() as u16);
        let mut i = 0;
        while i < locales.len() {
            let (locale, files) = locales[i];
            w.write_str(locale);
            w.write_u16_le(files.len() as u16);
            let mut j = 0;
            while j < files.len() {
                w.write_str(files[j]);
                j += 1;
            }
            i += 1;
        }
        out
    }

    /// Reads the record at the start of `bytes`, or returns `None` when it is
    /// not one.
    #[must_use]
    pub fn decode(bytes: &'a [u8]) -> Option<Self> {
        let mut r = ConstReader::new(bytes);
        let id = Self::decode_id(&mut r)?;
        let source_file = r.read_str()?;
        let line = r.read_u32_le()?;
        let pattern = r.read_str()?;
        let count = r.read_u16_le()?;
        let locales = (0..count)
            .map(|_| {
                let locale = r.read_str()?;
                let count = r.read_u16_le()?;
                let files = (0..count).map(|_| r.read_str()).collect::<Option<_>>()?;
                Some(RawNamespaceLocale { locale, files })
            })
            .collect::<Option<_>>()?;
        Some(Self {
            id,
            source_file,
            line,
            pattern,
            locales,
        })
    }

    /// Reads the ID of the namespace.
    pub(crate) fn decode_id(r: &mut ConstReader<'a>) -> Option<&'a str> {
        if r.read_bytes(PREFIX_LEN)? != prefix(NAMESPACE) {
            return None;
        }
        r.read_str()
    }

    /// Returns the records of every namespace in a compiled binary.
    #[cfg(feature = "check")]
    #[must_use]
    pub fn find_in_binary(binary: &'a [u8]) -> Vec<Self> {
        find_in_binary(binary, NAMESPACE, Self::decode)
    }
}

/// Returns the records of `kind` in `binary`, decoded by `decode`.
#[cfg(feature = "check")]
fn find_in_binary<'a, T>(
    binary: &'a [u8],
    kind: u8,
    decode: impl Fn(&'a [u8]) -> Option<T>,
) -> Vec<T> {
    memchr::memmem::find_iter(binary, &prefix(kind))
        .filter_map(|index| decode(&binary[index..]))
        .collect()
}

const fn str_len(s: &str) -> usize {
    2 + s.len()
}

const fn str_opt_len(s: Option<&str>) -> usize {
    match s {
        Some(s) => 1 + str_len(s),
        None => 1,
    }
}

/// The kind of a namespace record.
const NAMESPACE: u8 = b'N';
/// The kind of a translation record.
const TRANSLATION: u8 = b'T';

const PREFIX_KEY: u8 = 0xA7;

// "TOPCOAT_L10N" XOR'd byte by byte with PREFIX_KEY. Storing the scrambled form
// means the marker only appears in binaries that carry a record, not in every
// binary that links this crate.
const SCRAMBLED_PREFIX: [u8; 12] = [
    b'T' ^ PREFIX_KEY,
    b'O' ^ PREFIX_KEY,
    b'P' ^ PREFIX_KEY,
    b'C' ^ PREFIX_KEY,
    b'O' ^ PREFIX_KEY,
    b'A' ^ PREFIX_KEY,
    b'T' ^ PREFIX_KEY,
    b'_' ^ PREFIX_KEY,
    b'L' ^ PREFIX_KEY,
    b'1' ^ PREFIX_KEY,
    b'0' ^ PREFIX_KEY,
    b'N' ^ PREFIX_KEY,
];

/// The length of the marker and the kind that start every record.
const PREFIX_LEN: usize = SCRAMBLED_PREFIX.len() + 1;

/// Returns the marker that starts every record of `kind`.
const fn prefix(kind: u8) -> [u8; PREFIX_LEN] {
    let mut out = [kind; PREFIX_LEN];
    let mut i = 0;
    while i < SCRAMBLED_PREFIX.len() {
        out[i] = SCRAMBLED_PREFIX[i] ^ PREFIX_KEY;
        i += 1;
    }
    out
}

/// Creates the record of a namespace with the ID `$id` and nothing else, for
/// tests that register a namespace by hand.
#[cfg(test)]
macro_rules! test_namespace_record {
    ($id:expr) => {{
        const LEN: usize = $crate::RawNamespace::encoded_len($id, "", "", &[]);
        static RECORD: [u8; LEN] = $crate::RawNamespace::encode($id, "", 0, "", &[]);
        &RECORD
    }};
}

/// Creates the record of a `t!` call of the message `$id` and its optional
/// `$attribute` in the namespace `$namespace`, for tests that create a
/// translation by hand.
#[cfg(test)]
macro_rules! test_translation_record {
    ($namespace:expr, $id:expr, $attribute:expr) => {{
        const LEN: usize =
            $crate::RawTranslation::encoded_len($namespace, $id, $attribute, &[], file!());
        static RECORD: [u8; LEN] =
            $crate::RawTranslation::encode($namespace, $id, $attribute, &[], file!(), 0, 0);
        &RECORD
    }};
}

#[cfg(test)]
pub(crate) use {test_namespace_record, test_translation_record};

#[cfg(test)]
mod tests {
    use super::*;

    const ARGUMENTS: &[&str] = &["count", "name"];
    const TRANSLATION_LEN: usize =
        RawTranslation::encoded_len("ns", "items", Some("label"), ARGUMENTS, "src/lib.rs");
    static TRANSLATION_RECORD: [u8; TRANSLATION_LEN] =
        RawTranslation::encode("ns", "items", Some("label"), ARGUMENTS, "src/lib.rs", 12, 7);

    const LOCALES: &[(&str, &[&str])] = &[
        ("en", &["/app/l10n/en/a.ftl", "/app/l10n/en/b.ftl"]),
        ("de", &["/app/l10n/de/a.ftl"]),
    ];
    const NAMESPACE_LEN: usize =
        RawNamespace::encoded_len("ns", "/app/src/lib.rs", "l10n/{locale}/*.ftl", LOCALES);
    static NAMESPACE_RECORD: [u8; NAMESPACE_LEN] =
        RawNamespace::encode("ns", "/app/src/lib.rs", 3, "l10n/{locale}/*.ftl", LOCALES);

    #[test]
    fn decodes_what_a_translation_record_encodes() {
        assert_eq!(
            RawTranslation::decode(&TRANSLATION_RECORD),
            Some(RawTranslation {
                namespace: "ns",
                id: "items",
                attribute: Some("label"),
                arguments: vec!["count", "name"],
                file: "src/lib.rs",
                line: 12,
                column: 7,
            })
        );
    }

    #[test]
    fn decodes_what_a_namespace_record_encodes() {
        assert_eq!(
            RawNamespace::decode(&NAMESPACE_RECORD),
            Some(RawNamespace {
                id: "ns",
                source_file: "/app/src/lib.rs",
                line: 3,
                pattern: "l10n/{locale}/*.ftl",
                locales: vec![
                    RawNamespaceLocale {
                        locale: "en",
                        files: vec!["/app/l10n/en/a.ftl", "/app/l10n/en/b.ftl"],
                    },
                    RawNamespaceLocale {
                        locale: "de",
                        files: vec!["/app/l10n/de/a.ftl"],
                    },
                ],
            })
        );
    }

    #[test]
    fn a_record_of_one_kind_is_not_read_as_the_other() {
        assert_eq!(RawNamespace::decode(&TRANSLATION_RECORD), None);
        assert_eq!(RawTranslation::decode(&NAMESPACE_RECORD), None);
    }

    #[cfg(feature = "check")]
    #[test]
    fn finds_the_records_among_other_bytes() {
        let mut binary = b"some code".to_vec();
        binary.extend_from_slice(&TRANSLATION_RECORD);
        binary.extend_from_slice(b"more code");
        binary.extend_from_slice(&NAMESPACE_RECORD);
        binary.extend_from_slice(&TRANSLATION_RECORD);
        assert_eq!(RawTranslation::find_in_binary(&binary).len(), 2);
        assert_eq!(RawNamespace::find_in_binary(&binary).len(), 1);
    }

    #[cfg(feature = "check")]
    #[test]
    fn a_truncated_record_is_skipped() {
        let binary = &TRANSLATION_RECORD[..TRANSLATION_LEN - 1];
        assert_eq!(RawTranslation::find_in_binary(binary), []);
    }
}
