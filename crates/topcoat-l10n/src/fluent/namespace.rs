use std::{collections::HashMap, hint, sync::LazyLock};

use topcoat_core::cursor::ConstReader;

use crate::{Catalog, Locale, RawNamespace};

/// The Fluent files of one `messages!` invocation, registered so that its
/// `t!` can find them. Created by the expanded `messages!` macro.
#[doc(hidden)]
pub struct Namespace {
    record: &'static [u8],
    locales: &'static [NamespaceLocale],
}

impl Namespace {
    /// Creates the namespace described by `record`, a [`RawNamespace`], with
    /// the Fluent sources of each locale.
    #[must_use]
    pub const fn new(record: &'static [u8], locales: &'static [NamespaceLocale]) -> Self {
        Self { record, locales }
    }

    /// Returns the ID of the namespace, read from its record.
    ///
    /// # Panics
    ///
    /// Panics when the record is invalid. The records that `messages!`
    /// creates are always valid.
    fn id(&self) -> &'static str {
        // Tools find namespaces by searching the binary for their records.
        // Reading through `black_box` keeps the optimizer from removing them.
        let record = hint::black_box(self.record);
        RawNamespace::decode_id(&mut ConstReader::new(record))
            .expect("the namespace record is valid")
    }

    /// Returns the catalog of the namespace `id`, or `None` when no namespace
    /// with that ID is registered.
    ///
    /// The catalogs of all namespaces are built the first time this is called.
    ///
    /// # Panics
    ///
    /// Panics when a namespace has an invalid locale, or a source that
    /// [`Catalog::new`] rejects.
    #[must_use]
    pub fn catalog(id: &str) -> Option<&'static Catalog> {
        static CATALOGS: LazyLock<HashMap<&'static str, Catalog>> = LazyLock::new(|| {
            inventory::iter::<Namespace>()
                .map(|namespace| (namespace.id(), namespace.build()))
                .collect()
        });
        CATALOGS.get(id)
    }

    /// Returns the locales of the namespace `id`, as given by
    /// [`Catalog::locales`], or none when no namespace with that ID is
    /// registered.
    ///
    /// # Panics
    ///
    /// Panics like [`catalog`](Self::catalog).
    #[must_use]
    pub fn locales(id: &str) -> &'static [Locale] {
        match Self::catalog(id) {
            Some(catalog) => catalog.locales(),
            None => &[],
        }
    }

    /// Builds the catalog of the namespace.
    fn build(&self) -> Catalog {
        Catalog::new(self.locales.iter().map(|locale| {
            let id = Locale::try_from_str(locale.locale)
                .unwrap_or_else(|_| panic!("the locale `{}` is invalid", locale.locale));
            (id, locale.sources)
        }))
    }
}

inventory::collect!(Namespace);

/// The Fluent sources of one locale of a [`Namespace`].
#[doc(hidden)]
pub struct NamespaceLocale {
    locale: &'static str,
    sources: &'static [&'static str],
}

impl NamespaceLocale {
    /// Creates the sources of `locale`.
    #[must_use]
    pub const fn new(locale: &'static str, sources: &'static [&'static str]) -> Self {
        Self { locale, sources }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{locale, test_namespace_record};

    inventory::submit! {
        Namespace::new(
            test_namespace_record!("test"),
            &[
                NamespaceLocale::new("en", &["greeting = Hello\n"]),
                NamespaceLocale::new("de", &["greeting = Hallo\n"]),
            ],
        )
    }

    #[test]
    fn returns_the_catalog_of_a_registered_namespace() {
        let catalog = Namespace::catalog("test").expect("the namespace is registered");
        assert_eq!(
            catalog
                .format(&locale!("de"), "greeting", None, None)
                .as_deref(),
            Some("Hallo")
        );
    }

    #[test]
    fn returns_the_same_catalog_every_time() {
        assert!(std::ptr::eq(
            Namespace::catalog("test").unwrap(),
            Namespace::catalog("test").unwrap()
        ));
    }

    #[test]
    fn returns_none_for_an_unknown_namespace() {
        assert!(Namespace::catalog("unknown").is_none());
    }

    #[test]
    fn returns_the_locales_of_a_namespace() {
        assert_eq!(Namespace::locales("test"), [locale!("en"), locale!("de")]);
        assert_eq!(Namespace::locales("unknown"), [] as [Locale; 0]);
    }
}
