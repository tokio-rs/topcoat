// ---- Index emission test ----

// mdx_pages! emits MDX_INDEX_TESTS_FIXTURES_PAGES const and
// mdx_index_tests_fixtures_pages() accessor function.
// The pages/ fixture contains:
//   - hello-world.mdx (has frontmatter: title, date)
//   - about.mdx (no frontmatter)
//   - MyPost.mdx (no frontmatter)
//   - plain-markdown.md (no frontmatter)
//   - nested/deep-page.mdx (in subdirectory)
mod index_test {
    use topcoat_mdx_macro::mdx_pages;

    mdx_pages!("tests/fixtures/pages", prefix = "/index-test");

    #[test]
    fn mdx_index_emitted() {
        let index = mdx_index_tests_fixtures_pages();
        // We expect at least the top-level files: about, hello-world, MyPost, plain-markdown
        assert!(!index.is_empty(), "MDX_INDEX should not be empty");
    }

    #[test]
    fn mdx_index_entry_has_slug() {
        let index = mdx_index_tests_fixtures_pages();
        let hello = index
            .iter()
            .find(|e| e.slug == "hello-world")
            .expect("hello-world entry should exist");
        // hello-world.mdx has frontmatter with title "Hello World"
        assert_eq!(hello.title, Some("Hello World"));
    }

    #[test]
    fn mdx_index_entry_with_frontmatter() {
        let index = mdx_index_tests_fixtures_pages();
        let hello = index
            .iter()
            .find(|e| e.slug == "hello-world")
            .expect("hello-world entry should exist");
        assert_eq!(hello.date, Some("2024-06-15"));
        // hello-world.mdx has no tags in frontmatter
        assert!(hello.tags.is_empty());
    }

    #[test]
    fn mdx_index_entry_without_frontmatter() {
        let index = mdx_index_tests_fixtures_pages();
        let about = index
            .iter()
            .find(|e| e.slug == "about")
            .expect("about entry should exist");
        assert!(about.title.is_none());
        assert!(about.date.is_none());
        assert!(about.excerpt.is_none());
        assert!(about.tags.is_empty());
    }

    #[test]
    fn mdx_index_kebab_case_slug() {
        let index = mdx_index_tests_fixtures_pages();
        // MyPost.mdx should derive slug "my-post" via kebab-case
        let my_post = index
            .iter()
            .find(|e| e.slug == "my-post")
            .expect("my-post entry should exist for MyPost.mdx");
        assert!(my_post.title.is_none());
    }

    #[test]
    fn mdx_index_md_file_slug() {
        let index = mdx_index_tests_fixtures_pages();
        // plain-markdown.md should derive slug "plain-markdown"
        let plain = index
            .iter()
            .find(|e| e.slug == "plain-markdown")
            .expect("plain-markdown entry should exist");
        assert!(plain.title.is_none());
    }

    #[test]
    fn mdx_index_nested_path() {
        let index = mdx_index_tests_fixtures_pages();
        // nested/deep-page.mdx should derive path with subdirectory
        let deep = index
            .iter()
            .find(|e| e.slug == "deep-page")
            .expect("deep-page entry should exist");
        assert_eq!(deep.path, "/index-test/nested/deep-page");
    }

    #[test]
    fn mdx_index_flat_path() {
        let index = mdx_index_tests_fixtures_pages();
        // Top-level hello-world.mdx should have prefix + slug as path
        let hello = index
            .iter()
            .find(|e| e.slug == "hello-world")
            .expect("hello-world entry should exist");
        assert_eq!(hello.path, "/index-test/hello-world");
    }
}

// ---- Frontmatter tags reaching the index ----

mod tags_test {
    use topcoat_mdx_macro::mdx_pages;

    mdx_pages!("tests/fixtures/tagged-pages", prefix = "/tags-test");

    #[test]
    fn mdx_index_entry_with_several_tags() {
        let index = mdx_index_tests_fixtures_tagged_pages();
        let post = index
            .iter()
            .find(|e| e.slug == "multi-tag")
            .expect("multi-tag entry should exist");
        assert_eq!(post.tags, &["rust", "mdx", "web"]);
        assert_eq!(post.excerpt, Some("A post carrying several tags."));
    }

    #[test]
    fn mdx_index_entry_with_one_tag() {
        let index = mdx_index_tests_fixtures_tagged_pages();
        let post = index
            .iter()
            .find(|e| e.slug == "single-tag")
            .expect("single-tag entry should exist");
        assert_eq!(post.tags, &["rust"]);
    }
}

// ---- Custom frontmatter metadata reaching the index ----

// The four convenience fields cover the common case. Everything else a page
// declares reaches the consumer through `frontmatter_raw`, tagged with the
// syntax it was written in so the right deserializer can be picked.
mod custom_meta_test {
    use serde::Deserialize;
    use topcoat::mdx::MdxFrontmatterFormat;
    use topcoat_mdx_macro::mdx_pages;

    mdx_pages!(
        "tests/fixtures/custom-metadata",
        prefix = "/custom-meta-test"
    );

    #[derive(Deserialize)]
    struct BlogPostMeta {
        title: String,
        subtitle: String,
        #[serde(rename = "publishDate")]
        publish_date: String,
        #[serde(rename = "lastModifiedDate")]
        last_modified_date: String,
        tags: Vec<String>,
        excerpt: String,
        keywords: Vec<String>,
    }

    #[derive(Deserialize)]
    struct TomlMeta {
        subtitle: String,
        my_field: String,
        nested: TomlNested,
    }

    #[derive(Deserialize)]
    struct TomlNested {
        key: String,
    }

    #[derive(Deserialize)]
    struct ComplexMeta {
        draft: bool,
        priority: u32,
        score: f64,
        nested: ComplexNested,
    }

    #[derive(Deserialize)]
    struct ComplexNested {
        count: u32,
    }

    #[derive(Deserialize)]
    struct TitleOnly {
        title: String,
    }

    // Names a field no fixture declares. It exists to fail deserialization,
    // so it is never read.
    #[derive(Deserialize)]
    struct RequiresMissingField {
        #[allow(dead_code)]
        author: String,
    }

    fn entry(slug: &str) -> &'static topcoat::mdx::MdxIndexEntry {
        mdx_index_tests_fixtures_custom_metadata()
            .iter()
            .find(|e| e.slug == slug)
            .unwrap_or_else(|| panic!("{slug} entry should exist"))
    }

    #[test]
    fn mdx_index_yaml_metadata_extracted() {
        let meta: BlogPostMeta = serde_saphyr::from_str(entry("example-post").frontmatter_raw)
            .expect("blog post frontmatter should deserialize");
        assert_eq!(meta.title, "Blog Post with Custom Metadata");
        assert_eq!(meta.subtitle, "A subtitle for the post");
        assert_eq!(meta.publish_date, "2025-01-01");
        assert_eq!(meta.last_modified_date, "2025-06-01");
        assert_eq!(
            meta.excerpt,
            "An excerpt summarizing the blog post content."
        );
        assert_eq!(meta.tags, ["blog", "example", "test"]);
        assert_eq!(
            meta.keywords,
            ["blog", "example", "metadata", "keywords", "test"]
        );
    }

    #[test]
    fn mdx_index_toml_metadata_extracted() {
        let meta: TomlMeta = toml::from_str(entry("toml-post").frontmatter_raw)
            .expect("TOML frontmatter should deserialize");
        assert_eq!(meta.subtitle, "Using TOML frontmatter");
        assert_eq!(meta.my_field, "my_value");
        assert_eq!(meta.nested.key, "value");
    }

    // Values that are not strings survive the round trip, which the four
    // `Option<&str>` convenience fields cannot express at all.
    #[test]
    fn mdx_index_typed_scalars_extracted() {
        let meta: ComplexMeta = serde_saphyr::from_str(entry("complex-post").frontmatter_raw)
            .expect("complex frontmatter should deserialize");
        assert!(!meta.draft);
        assert_eq!(meta.priority, 5);
        assert!((meta.score - 2.5).abs() < f64::EPSILON);
        assert_eq!(meta.nested.count, 42);
    }

    // A consumer may model only the fields it cares about.
    #[test]
    fn mdx_index_unknown_fields_ignored() {
        let meta: TitleOnly = serde_saphyr::from_str(entry("example-post").frontmatter_raw)
            .expect("a subset of the fields should deserialize");
        assert_eq!(meta.title, "Blog Post with Custom Metadata");
    }

    // Deserializing raw frontmatter is fallible, not fatal.
    #[test]
    fn mdx_index_missing_field_is_error() {
        let result: Result<RequiresMissingField, _> =
            serde_saphyr::from_str(entry("example-post").frontmatter_raw);
        assert!(result.is_err(), "a missing field should be an error");
    }

    #[test]
    fn mdx_index_custom_metadata_accessible() {
        let raw = entry("example-post").frontmatter_raw;
        for field in ["subtitle", "publishDate", "lastModifiedDate", "keywords"] {
            assert!(raw.contains(field), "{field} should reach the index");
        }
    }

    #[test]
    fn mdx_index_no_frontmatter_empty_raw() {
        let plain = entry("plain-post");
        assert!(plain.frontmatter_raw.is_empty());
        assert_eq!(plain.frontmatter_format, MdxFrontmatterFormat::None);
    }

    #[test]
    fn mdx_index_format_reported_yaml() {
        assert_eq!(
            entry("example-post").frontmatter_format,
            MdxFrontmatterFormat::Yaml
        );
    }

    #[test]
    fn mdx_index_format_reported_toml() {
        assert_eq!(
            entry("toml-post").frontmatter_format,
            MdxFrontmatterFormat::Toml
        );
    }

    // The parser strips the delimiters, so the syntax cannot be recovered from
    // the string itself. This is why the format travels alongside it.
    #[test]
    fn mdx_index_raw_has_no_delimiters() {
        for slug in ["example-post", "toml-post", "complex-post"] {
            let raw = entry(slug).frontmatter_raw;
            assert!(!raw.starts_with("---"), "{slug} should not keep ---");
            assert!(!raw.starts_with("+++"), "{slug} should not keep +++");
        }
    }

    // The frontmatter block runs to about thirty words; the body is ten.
    #[test]
    fn mdx_index_word_count_excludes_frontmatter() {
        assert_eq!(entry("example-post").word_count, 10);
    }

    #[test]
    fn mdx_index_word_count_no_frontmatter() {
        assert_eq!(entry("plain-post").word_count, 8);
    }
}

// ---- Typed frontmatter ----

// With `frontmatter = Type` the macro deserializes each page's frontmatter
// itself, once, and picks the deserializer from the syntax the page used.
mod typed_meta_test {
    use serde::Deserialize;
    use topcoat_mdx_macro::mdx_pages;

    // Every field is optional so that one type covers fixtures written in both
    // syntaxes, which is the point being tested.
    #[derive(Deserialize)]
    pub struct TestMeta {
        pub title: Option<String>,
        pub subtitle: Option<String>,
        #[serde(rename = "publishDate")]
        pub publish_date: Option<String>,
        pub my_field: Option<String>,
        pub keywords: Option<Vec<String>>,
    }

    mdx_pages!(
        "tests/fixtures/custom-metadata",
        prefix = "/typed-meta-test",
        frontmatter = TestMeta
    );

    // A second scan of the same directory without the argument, to check that
    // the untyped form still compiles alongside.
    mod untyped {
        use topcoat_mdx_macro::mdx_pages;

        mdx_pages!("tests/fixtures/tagged-pages", prefix = "/untyped-meta-test");

        #[test]
        fn mdx_index_untyped_has_no_meta() {
            for entry in mdx_index_tests_fixtures_tagged_pages() {
                assert!(entry.meta().is_none(), "no frontmatter type was given");
            }
        }
    }

    fn entry(slug: &str) -> &'static topcoat::mdx::MdxIndexEntry<TestMeta> {
        mdx_index_tests_fixtures_custom_metadata()
            .iter()
            .find(|e| e.slug == slug)
            .unwrap_or_else(|| panic!("{slug} entry should exist"))
    }

    #[test]
    fn mdx_index_typed_meta_yaml_values() {
        let meta = entry("example-post")
            .meta()
            .expect("YAML post has metadata");
        assert_eq!(
            meta.title.as_deref(),
            Some("Blog Post with Custom Metadata")
        );
        assert_eq!(meta.subtitle.as_deref(), Some("A subtitle for the post"));
        assert_eq!(meta.publish_date.as_deref(), Some("2025-01-01"));
        assert_eq!(meta.keywords.as_ref().map(Vec::len), Some(5));
    }

    // The macro dispatched to the TOML deserializer without the consumer
    // saying anything about the syntax.
    #[test]
    fn mdx_index_typed_meta_toml_values() {
        let meta = entry("toml-post").meta().expect("TOML post has metadata");
        assert_eq!(meta.title.as_deref(), Some("TOML Custom Fields"));
        assert_eq!(meta.subtitle.as_deref(), Some("Using TOML frontmatter"));
        assert_eq!(meta.my_field.as_deref(), Some("my_value"));
    }

    // A frontmatter type does not oblige every page in the directory to carry
    // frontmatter.
    #[test]
    fn mdx_index_typed_meta_absent_without_frontmatter() {
        assert!(entry("plain-post").meta().is_none());
        assert!(entry("example-post").meta().is_some());
    }

    // Parsing happens once, not on every read.
    #[test]
    fn mdx_index_typed_meta_parsed_once() {
        let first = entry("example-post").meta().expect("metadata is present");
        let second = entry("example-post").meta().expect("metadata is present");
        assert!(std::ptr::eq(first, second), "meta() should not reparse");
    }
}

// ---- Frontmatter that does not match its type ----

// The macro cannot check a page against the type it was given: it sees only
// the type's path, and serde does not run while the macro expands. The
// mismatch therefore surfaces on first read, naming the file it came from.
mod mismatched_meta_test {
    use serde::Deserialize;
    use topcoat_mdx_macro::mdx_pages;

    #[derive(Deserialize)]
    pub struct RequiresAuthor {
        #[allow(dead_code)]
        pub author: String,
    }

    mdx_pages!(
        "tests/fixtures/mismatched-meta",
        prefix = "/mismatched-meta-test",
        frontmatter = RequiresAuthor
    );

    #[test]
    #[should_panic(expected = "bad-post.mdx")]
    fn mdx_index_mismatched_meta_panics_naming_the_file() {
        let entry = mdx_index_tests_fixtures_mismatched_meta()
            .iter()
            .find(|entry| entry.slug == "bad-post")
            .expect("bad-post entry should exist");
        let _ = entry.meta();
    }
}

// ---- index.mdx standing for its directory ----

// A post that keeps its assets in a directory of its own should not have a
// route ending in a repeated segment.
mod index_file_test {
    use topcoat_mdx_macro::mdx_pages;

    mdx_pages!("tests/fixtures/index-pages", prefix = "/index-file-test");

    fn path_of(slug: &str) -> &'static str {
        mdx_index_tests_fixtures_index_pages()
            .iter()
            .find(|entry| entry.slug == slug)
            .unwrap_or_else(|| panic!("{slug} entry should exist"))
            .path
    }

    #[test]
    fn index_mdx_takes_its_directory_route() {
        assert_eq!(path_of("my-post"), "/index-file-test/my-post");
    }

    // The same holds for `.md`, at any depth.
    #[test]
    fn nested_index_md_takes_its_directory_route() {
        assert_eq!(path_of("old-post"), "/index-file-test/archive/old-post");
    }

    // An index file is named after the directory it stands for. Slugs would
    // otherwise all be "index" and collide with each other.
    #[test]
    fn index_file_slug_is_its_directory() {
        let slugs: Vec<&str> = mdx_index_tests_fixtures_index_pages()
            .iter()
            .map(|entry| entry.slug)
            .collect();
        assert!(!slugs.contains(&"index"), "found {slugs:?}");
        assert!(slugs.contains(&"my-post"), "found {slugs:?}");
        assert!(slugs.contains(&"old-post"), "found {slugs:?}");
    }

    #[test]
    fn sibling_of_an_index_file_is_unaffected() {
        assert_eq!(path_of("appendix"), "/index-file-test/my-post/appendix");
    }

    #[test]
    fn a_flat_file_is_unaffected() {
        assert_eq!(path_of("flat"), "/index-file-test/flat");
    }

    // No route ends in the literal segment, which is the point of the rule.
    #[test]
    fn no_route_ends_in_index() {
        for entry in mdx_index_tests_fixtures_index_pages() {
            assert!(
                !entry.path.ends_with("/index"),
                "{} should have collapsed",
                entry.path
            );
        }
    }
}

// ---- Index entry type test ----

mod type_test {
    use topcoat::mdx::{MdxFrontmatterFormat, MdxIndexEntry};

    static TEST_TAGS: &[&str] = &["tag1"];

    #[test]
    fn mdx_index_entry_fields() {
        // Verify MdxIndexEntry has the expected fields. The annotation picks
        // up the default type parameter, since no frontmatter type is in play.
        let entry: MdxIndexEntry = MdxIndexEntry {
            slug: "test",
            path: "/blog/test",
            title: Some("Test Title"),
            date: Some("2024-01-01"),
            excerpt: Some("Test excerpt"),
            tags: TEST_TAGS,
            frontmatter_raw: "title: Test Title\nsubtitle: Test Subtitle",
            frontmatter_format: MdxFrontmatterFormat::Yaml,
            word_count: 42,
            meta_fn: None,
        };
        assert_eq!(entry.slug, "test");
        assert_eq!(entry.path, "/blog/test");
        assert_eq!(entry.title, Some("Test Title"));
        assert_eq!(entry.date, Some("2024-01-01"));
        assert_eq!(entry.excerpt, Some("Test excerpt"));
        assert_eq!(entry.tags, &["tag1"]);
        assert!(entry.frontmatter_raw.contains("subtitle"));
        assert_eq!(entry.frontmatter_format, MdxFrontmatterFormat::Yaml);
        assert_eq!(entry.word_count, 42);
    }

    #[test]
    fn mdx_index_entry_empty_optional_fields() {
        static EMPTY_TAGS: &[&str] = &[];
        let entry: MdxIndexEntry = MdxIndexEntry {
            slug: "minimal",
            path: "/blog/minimal",
            title: None,
            date: None,
            excerpt: None,
            tags: EMPTY_TAGS,
            frontmatter_raw: "",
            frontmatter_format: MdxFrontmatterFormat::None,
            word_count: 0,
            meta_fn: None,
        };
        assert!(entry.title.is_none());
        assert!(entry.date.is_none());
        assert!(entry.excerpt.is_none());
        assert!(entry.tags.is_empty());
        assert!(entry.frontmatter_raw.is_empty());
        assert_eq!(entry.frontmatter_format, MdxFrontmatterFormat::None);
        assert_eq!(entry.word_count, 0);
    }
}
