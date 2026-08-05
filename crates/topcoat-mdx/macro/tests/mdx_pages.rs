use topcoat_mdx_macro::mdx_pages;

// ---- Basic mdx_pages! compilation ----

// mdx_pages! scans the fixtures/pages directory and registers each .mdx file.
mdx_pages!("tests/fixtures/pages", prefix = "/blog");

#[test]
fn mdx_pages_compiles() {
    // The fact that this test module compiles proves that mdx_pages!
    // successfully scanned the directory and generated valid registration code.
}

// ---- Nested directory ----

// Nested .mdx files should produce routes like /docs/nested/deep-page.
mod nested_test {
    use topcoat_mdx_macro::mdx_pages;

    mdx_pages!("tests/fixtures/pages/nested", prefix = "/docs");

    #[test]
    fn mdx_pages_nested_directory() {
        // Compilation proves nested file was discovered and registered.
    }
}

// ---- Empty directory ----

// mdx_pages! with an empty directory should compile without error.
mod empty_dir_test {
    use topcoat_mdx_macro::mdx_pages;

    // Create an empty directory for this test.
    mdx_pages!("tests/fixtures/empty_pages");

    #[test]
    fn mdx_pages_empty_directory() {
        // Compilation succeeds even with no .mdx files found.
    }
}

// ---- .md-only directory ----

// mdx_pages! with only .md files (no .mdx) should compile and register them.
mod md_only_test {
    use topcoat_mdx_macro::mdx_pages;

    mdx_pages!("tests/fixtures/md-only-pages", prefix = "/md-only");

    #[test]
    fn mdx_pages_discovers_md_files() {
        // Compilation proves .md files were registered.
        // The directory contains only .md files, no .mdx files.
    }
}

// ---- mdx_pages! with wrapper ----

mod wrapper_test {
    use topcoat::view::{View, component, view};
    use topcoat_mdx_macro::mdx_pages;

    type Result<T = View> = topcoat::Result<T>;

    #[component]
    async fn blog_wrapper(#[default] child: View) -> Result {
        view! { <article class="blog-layout">(child)</article> }
    }

    // mdx_pages! with wrapper = Path should compile.
    mdx_pages!(
        "tests/fixtures/pages",
        prefix = "/with-wrapper",
        wrapper = blog_wrapper
    );

    #[test]
    fn mdx_pages_with_wrapper_compiles() {
        // Compilation proves wrapper tokens were generated.
    }
}

// ---- mdx_pages! forwarding frontmatter to the wrapper ----

// With `frontmatter = Type`, a wrapper takes the parsed metadata as a prop, so
// a page header can be rendered from frontmatter the body never mentions.
mod wrapper_frontmatter_test {
    use serde::Deserialize;
    use topcoat::{
        context::CxTestBuilder,
        view::{View, component, view},
    };
    use topcoat_mdx_macro::mdx_pages;

    type Result<T = View> = topcoat::Result<T>;

    #[derive(Deserialize)]
    pub struct PostMeta {
        pub subtitle: Option<String>,
    }

    // `meta` is an `Option` because a directory may hold pages that carry no
    // frontmatter at all; those pass `None` rather than being rejected.
    #[component]
    async fn post_wrapper(#[default] child: View, meta: Option<&'static PostMeta>) -> Result {
        view! {
            <article>
                if let Some(subtitle) = meta
                    .and_then(|meta| meta.subtitle.as_deref()) {
                    <p class="subtitle">(subtitle)</p>
                }
                (child)
            </article>
        }
    }

    mdx_pages!(
        "tests/fixtures/custom-metadata",
        prefix = "/wrapper-frontmatter",
        wrapper = post_wrapper,
        frontmatter = PostMeta
    );

    #[tokio::test]
    async fn wrapper_renders_frontmatter_the_body_does_not_carry() {
        let meta = mdx_index_tests_fixtures_custom_metadata()
            .iter()
            .find(|entry| entry.slug == "example-post")
            .and_then(topcoat::mdx::MdxIndexEntry::meta)
            .expect("the YAML fixture has metadata");

        // `view!` renders components through a context named `__cx`, the same
        // name the macro-generated route handlers bind.
        let cx = CxTestBuilder::new().build();
        let __cx = &cx;
        let body = view! { <p>"Body content"</p> }.expect("body renders");
        let view = view! { post_wrapper(child: body, meta: Some(meta)) }.expect("wrapper renders");

        let html = view.render(&cx);
        assert!(
            html.contains("A subtitle for the post"),
            "the subtitle only exists in frontmatter, so it proves the wrapper received it"
        );
        assert!(html.contains("Body content"), "the body still renders");
    }
}

// ---- mdx_pages! with components and overrides ----

#[allow(dead_code)]
mod components_and_overrides_test {
    use topcoat::view::{View, component, view};
    use topcoat_mdx_macro::mdx_pages;

    type Result<T = View> = topcoat::Result<T>;

    #[component]
    async fn callout_component(#[default] child: View) -> Result {
        view! { <div class="callout">(child)</div> }
    }

    #[component]
    async fn custom_link_override(href: &'static str, #[default] child: View) -> Result {
        view! { <a class="custom-link" href=(href)>(child)</a> }
    }

    // mdx_pages! with components = {...} and overrides = {...} should compile.
    mdx_pages!(
        "tests/fixtures/pages",
        prefix = "/with-components",
        components = { Callout => callout_component },
        overrides = { "a" => custom_link_override }
    );

    #[test]
    fn mdx_pages_with_components_and_overrides_compiles() {
        // Compilation proves components and overrides were threaded through.
    }
}
