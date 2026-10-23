use topcoat_core::cursor::{ConstReader, ConstWriter};

/// A record of a `t!` call stored in a compiled binary.
///
/// The generated [`Translation`](crate::Translation) reads its key from this record.
/// Tools can use [`find_in_binary`](Self::find_in_binary) to find calls that remain
/// after compilation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTranslation<'a> {
    /// The namespace containing this translation call.
    pub namespace: &'a str,
    /// The translated message's ID.
    pub id: &'a str,
    /// The selected message attribute, if present.
    pub attribute: Option<&'a str>,
    /// Argument names supplied by the caller.
    pub arguments: Vec<&'a str>,
    /// The caller's source file path from `file!()`.
    pub file: &'a str,
    /// The key's line number, counted from 1.
    pub line: u32,
    /// The key's column number, counted from 1.
    pub column: u32,
}

impl<'a> RawTranslation<'a> {
    /// Calculates the byte length needed by [`encode`](Self::encode).
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

    /// Encodes a translation call as a byte record. Set `N` to
    /// [`encoded_len`](Self::encoded_len) for the same inputs.
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

    /// Decodes a record at the start of `bytes`, returning `None` if no valid record is
    /// present.
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

    /// Reads the namespace ID, message ID and optional attribute from a record.
    pub(crate) fn decode_key(
        r: &mut ConstReader<'a>,
    ) -> Option<(&'a str, &'a str, Option<&'a str>)> {
        if r.read_bytes(PREFIX_LEN)? != prefix(TRANSLATION) {
            return None;
        }
        Some((r.read_str()?, r.read_str()?, r.read_str_opt()?))
    }

    /// Finds translation call records in a compiled binary.
    #[cfg(feature = "check")]
    #[must_use]
    pub fn find_in_binary(binary: &'a [u8]) -> Vec<Self> {
        find_in_binary(binary, TRANSLATION, Self::decode)
    }
}

/// A record of a `messages!` call stored in a compiled binary.
///
/// Use [`find_in_binary`](Self::find_in_binary) to find the namespace records generated
/// by the macro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawNamespace<'a> {
    /// The namespace's unique ID.
    pub id: &'a str,
    /// The absolute path of the file containing `messages!`.
    pub source_file: &'a str,
    /// The call's line number, counted from 1.
    pub line: u32,
    /// The call's column number, counted from 1.
    pub column: u32,
    /// The file pattern supplied to the macro.
    pub pattern: &'a str,
    /// Locales and the source files embedded for each one.
    pub locales: Vec<RawNamespaceLocale<'a>>,
}

/// Embedded file paths for a locale in a [`RawNamespace`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawNamespaceLocale<'a> {
    /// The locale name captured by the file pattern.
    pub locale: &'a str,
    /// Absolute paths of the embedded files.
    pub files: Vec<&'a str>,
}

impl<'a> RawNamespace<'a> {
    /// Calculates the byte length needed by [`encode`](Self::encode).
    #[doc(hidden)]
    #[must_use]
    pub const fn encoded_len(
        id: &str,
        source_file: &str,
        pattern: &str,
        locales: &[(&str, &[&str])],
    ) -> usize {
        let mut len = PREFIX_LEN + str_len(id) + str_len(source_file) + 8 + str_len(pattern) + 2;
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

    /// Encodes a namespace call as a byte record. Set `N` to
    /// [`encoded_len`](Self::encoded_len) for the same inputs.
    #[doc(hidden)]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn encode<const N: usize>(
        id: &str,
        source_file: &str,
        line: u32,
        column: u32,
        pattern: &str,
        locales: &[(&str, &[&str])],
    ) -> [u8; N] {
        let mut out = [0; N];
        let mut w = ConstWriter::new(&mut out);
        w.write_bytes(&prefix(NAMESPACE));
        w.write_str(id);
        w.write_str(source_file);
        w.write_u32_le(line);
        w.write_u32_le(column);
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

    /// Decodes a record at the start of `bytes`, returning `None` if no valid record is
    /// present.
    #[must_use]
    pub fn decode(bytes: &'a [u8]) -> Option<Self> {
        let mut r = ConstReader::new(bytes);
        let id = Self::decode_id(&mut r)?;
        let source_file = r.read_str()?;
        let line = r.read_u32_le()?;
        let column = r.read_u32_le()?;
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
            column,
            pattern,
            locales,
        })
    }

    /// Reads the namespace ID from a record.
    pub(crate) fn decode_id(r: &mut ConstReader<'a>) -> Option<&'a str> {
        if r.read_bytes(PREFIX_LEN)? != prefix(NAMESPACE) {
            return None;
        }
        r.read_str()
    }

    /// Finds namespace records in a compiled binary.
    #[cfg(feature = "check")]
    #[must_use]
    pub fn find_in_binary(binary: &'a [u8]) -> Vec<Self> {
        find_in_binary(binary, NAMESPACE, Self::decode)
    }
}

/// Finds records of the requested kind and decodes them with `decode`.
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

/// Record type marker for namespaces.
const NAMESPACE: u8 = b'N';
/// Record type marker for translations.
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

/// Byte length of a record's opening marker and type.
const PREFIX_LEN: usize = SCRAMBLED_PREFIX.len() + 1;

/// Builds the opening marker for the given record type.
const fn prefix(kind: u8) -> [u8; PREFIX_LEN] {
    let mut out = [kind; PREFIX_LEN];
    let mut i = 0;
    while i < SCRAMBLED_PREFIX.len() {
        out[i] = SCRAMBLED_PREFIX[i] ^ PREFIX_KEY;
        i += 1;
    }
    out
}

/// Creates a minimal namespace record with ID `$id` for tests that register namespaces
/// directly.
#[cfg(test)]
macro_rules! test_namespace_record {
    ($id:expr) => {{
        const LEN: usize = $crate::RawNamespace::encoded_len($id, "", "", &[]);
        static RECORD: [u8; LEN] = $crate::RawNamespace::encode($id, "", 0, 0, "", &[]);
        &RECORD
    }};
}

/// Creates a translation record for tests, using message `$id`, optional `$attribute`
/// and namespace `$namespace`.
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
    static NAMESPACE_RECORD: [u8; NAMESPACE_LEN] = RawNamespace::encode(
        "ns",
        "/app/src/lib.rs",
        3,
        5,
        "l10n/{locale}/*.ftl",
        LOCALES,
    );

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
                column: 5,
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
