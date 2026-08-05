#![doc = include_str!("../docs/module.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use topcoat_mdx_macro::{compile_mdx, mdx_page, mdx_pages};

/// An index entry for a single `.mdx` or `.md` page discovered by `mdx_pages!`.
///
/// Used to build structured indexes (blog listings, sitemaps, tag pages) from
/// MDX frontmatter and file path metadata at compile time.
///
/// The named fields cover the frontmatter every page tends to carry. Pages
/// that declare more reach the consumer through [`frontmatter_raw`], which
/// holds the whole block for deserializing into a type of your own.
///
/// Passing `frontmatter = Type` to `mdx_pages!` goes further: the whole block
/// is deserialized into that type once per page, reachable through [`meta`].
/// `M` is that type, and defaults to `()` when the argument is not given.
///
/// Entries are built by `mdx_pages!`. Constructing one by hand is possible but
/// not the intended use, so a new field is a breaking change for any code that
/// does.
///
/// [`frontmatter_raw`]: Self::frontmatter_raw
/// [`meta`]: Self::meta
pub struct MdxIndexEntry<M: 'static = ()> {
    /// The kebab-cased route slug derived from the file path stem.
    pub slug: &'static str,
    /// The full route path including any prefix and subdirectory structure
    /// (e.g. `"/blog/updates/roadmap"`). Use this for generating links.
    pub path: &'static str,
    /// The `title` field from frontmatter, if present.
    pub title: Option<&'static str>,
    /// The `date` field from frontmatter, if present.
    pub date: Option<&'static str>,
    /// The `excerpt` field from frontmatter, if present.
    pub excerpt: Option<&'static str>,
    /// The `tags` field from frontmatter as a slice of strings, empty if absent.
    pub tags: &'static [&'static str],
    /// The whole frontmatter block, with the `---` or `+++` delimiters already
    /// stripped. Empty when the page carries no frontmatter.
    ///
    /// Deserialize it into your own type to read fields beyond the named ones,
    /// picking the deserializer from [`frontmatter_format`]:
    ///
    /// ```no_run
    /// # use topcoat_mdx::{MdxFrontmatterFormat, MdxIndexEntry};
    /// # #[derive(serde::Deserialize)]
    /// # struct PostMeta { subtitle: String }
    /// # fn parse(entry: &MdxIndexEntry) -> Option<PostMeta> {
    /// match entry.frontmatter_format {
    ///     MdxFrontmatterFormat::Yaml => serde_saphyr::from_str(entry.frontmatter_raw).ok(),
    ///     MdxFrontmatterFormat::Toml => toml::from_str(entry.frontmatter_raw).ok(),
    ///     MdxFrontmatterFormat::None => None,
    /// }
    /// # }
    /// ```
    ///
    /// [`frontmatter_format`]: Self::frontmatter_format
    pub frontmatter_raw: &'static str,
    /// The syntax [`frontmatter_raw`] is written in.
    ///
    /// The delimiters are stripped during parsing, so the syntax cannot be
    /// recovered from the string alone. Read this instead of guessing.
    ///
    /// [`frontmatter_raw`]: Self::frontmatter_raw
    pub frontmatter_format: MdxFrontmatterFormat,
    /// Whitespace-separated words in the page body, counted when the page was
    /// compiled and excluding the frontmatter block.
    ///
    /// Code blocks and component markup count toward the total, matching what
    /// reading-time tooling reports for a markdown file. Turn it into an
    /// estimate with a rate of your choosing:
    ///
    /// ```
    /// # let word_count = 400_usize;
    /// let minutes = word_count.div_ceil(200);
    /// # assert_eq!(minutes, 2);
    /// ```
    pub word_count: usize,
    /// Reads the parsed frontmatter. Set by `mdx_pages!` when it was given a
    /// `frontmatter = Type` argument and the page carries frontmatter.
    ///
    /// Held as a function rather than a reference because the index is a
    /// `const` and the parsed value is not available until first use.
    #[doc(hidden)]
    pub meta_fn: Option<fn() -> &'static M>,
}

impl<M: 'static> MdxIndexEntry<M> {
    /// The page's frontmatter, deserialized into the type passed as
    /// `frontmatter = Type`.
    ///
    /// `None` when `mdx_pages!` was called without that argument, or when the
    /// page carries no frontmatter at all.
    ///
    /// The first call parses the block; later calls return the same value. A
    /// page whose frontmatter does not match the type panics here, naming the
    /// file, since the mismatch cannot be caught while the macro expands.
    #[must_use]
    pub fn meta(&self) -> Option<&'static M> {
        self.meta_fn.map(|read| read())
    }
}

// Written by hand so that the type of the parsed frontmatter does not have to
// implement these itself. The field holding it is a function pointer, which is
// `Copy` and `Debug` whatever `M` turns out to be.
impl<M: 'static> Clone for MdxIndexEntry<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: 'static> Copy for MdxIndexEntry<M> {}

impl<M: 'static> core::fmt::Debug for MdxIndexEntry<M> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MdxIndexEntry")
            .field("slug", &self.slug)
            .field("path", &self.path)
            .field("title", &self.title)
            .field("date", &self.date)
            .field("excerpt", &self.excerpt)
            .field("tags", &self.tags)
            .field("frontmatter_raw", &self.frontmatter_raw)
            .field("frontmatter_format", &self.frontmatter_format)
            .field("word_count", &self.word_count)
            .field("has_meta", &self.meta_fn.is_some())
            .finish()
    }
}

/// The syntax a page's frontmatter block is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MdxFrontmatterFormat {
    /// The page carries no frontmatter.
    None,
    /// YAML frontmatter, written between `---` delimiters.
    Yaml,
    /// TOML frontmatter, written between `+++` delimiters.
    Toml,
}

/// Support code called by generated `mdx_pages!` output. Not a public API.
///
/// Behind the `frontmatter` feature because this is the only part of the crate
/// that parses frontmatter at runtime. Rendering MDX resolves frontmatter while
/// the macro expands, so a site that does not name a frontmatter type never
/// reaches this module and should not carry its dependencies.
///
/// Generated code calls these rather than naming `serde_saphyr` or `toml`
/// directly, so that a consumer does not have to depend on either, and so the
/// panic can name the file the frontmatter came from.
#[cfg(feature = "frontmatter")]
#[doc(hidden)]
pub mod __private {
    /// Deserializes a page's YAML frontmatter, naming the file when it does
    /// not match the requested type.
    ///
    /// # Panics
    ///
    /// When the frontmatter does not deserialize into `T`.
    #[must_use]
    pub fn parse_yaml<T: serde::de::DeserializeOwned>(raw: &str, file: &str) -> T {
        serde_saphyr::from_str(raw).unwrap_or_else(|error| {
            panic!("YAML frontmatter of '{file}' does not match its `frontmatter` type: {error}")
        })
    }

    /// Deserializes a page's TOML frontmatter, naming the file when it does
    /// not match the requested type.
    ///
    /// # Panics
    ///
    /// When the frontmatter does not deserialize into `T`.
    #[must_use]
    pub fn parse_toml<T: serde::de::DeserializeOwned>(raw: &str, file: &str) -> T {
        toml::from_str(raw).unwrap_or_else(|error| {
            panic!("TOML frontmatter of '{file}' does not match its `frontmatter` type: {error}")
        })
    }
}

#[doc = include_str!("../macro/docs/mdx_components.md")]
#[macro_export]
macro_rules! mdx_components {
    ($($name:ident => $path:path),* $(,)?) => {
        { $($name => $path),* }
    };
}
