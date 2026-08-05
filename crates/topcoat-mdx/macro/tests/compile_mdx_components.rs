#![allow(clippy::approx_constant)]

use topcoat::{context::CxTestBuilder, mdx::compile_mdx, view as topcoat_view_module};
use topcoat_view_module::{View, component, view};

type Result<T = View> = topcoat::Result<T>;

/// Helper to run `compile_mdx`! with a `__cx` binding so that component
/// code (`__view(__cx, ...)`) compiles correctly.
///
/// The `compile_mdx!` macro generates code that references `__cx` when
/// the MDX content contains components. This wrapper provides the binding.
macro_rules! compile_mdx_with_cx {
    ( $cx:expr => $( $arg:tt )* ) => {{
        let __cx = &$cx;
        compile_mdx!($( $arg )*)
    }};
}

// ---------------------------------------------------------------------------
// Mock components for integration tests
// ---------------------------------------------------------------------------

mod mock {
    use super::*;

    // --- Callout: var prop ---
    #[component]
    pub async fn callout(var: &'static str, #[default] child: View) -> Result {
        view! { <div class="mdx-callout" data-var=(var)>(child)</div> }
    }

    // --- Divider: no props ---
    #[component]
    pub async fn divider() -> Result {
        view! { <hr class="mdx-divider" /> }
    }

    // --- Badge: label prop ---
    #[component]
    pub async fn badge(label: &'static str) -> Result {
        view! { <span class="mdx-badge">(label)</span> }
    }

    // --- Wrapper: child content ---
    #[component]
    pub async fn wrapper(#[default] child: View) -> Result {
        view! { <section class="mdx-wrapper">(child)</section> }
    }

    // --- NestedOuter: name prop + child ---
    #[component]
    pub async fn nested_outer(name: &'static str, #[default] child: View) -> Result {
        view! { <div class="mdx-nested-outer" data-name=(name)>(child)</div> }
    }

    // --- NestedInner: count prop + child ---
    #[component]
    pub async fn nested_inner(count: i64, #[default] child: View) -> Result {
        view! {
            <div class="mdx-nested-inner" data-count=(count.to_string())>(child)</div>
        }
    }

    // --- Config: multiple prop types ---
    #[component]
    pub async fn config(enabled: bool, count: i64, ratio: f64, label: &'static str) -> Result {
        view! {
            <div
                class="mdx-config"
                data-enabled=(enabled.to_string())
                data-count=(count.to_string())
                data-ratio=(ratio.to_string())
                data-label=(label)
            ></div>
        }
    }

    // --- BareAttr: boolean prop ---
    #[component]
    pub async fn bare_attr(#[default] dismissible: bool) -> Result {
        view! {
            <div class="mdx-bare-attr" data-dismissible=(dismissible.to_string())></div>
        }
    }
}

// ---------------------------------------------------------------------------
// Task 1: compile_mdx! two-arg parsing, WalkContext wiring, error emission
// ---------------------------------------------------------------------------

// --- components_basic: paragraph + component with string prop ---

mod components_basic {
    use super::*;

    #[tokio::test]
    async fn compiles() {
        // Verify the two-arg compile_mdx! macro expands and compiles.
        let cx = CxTestBuilder::new().build();
        let _view = compile_mdx_with_cx!(cx =>
            mdx_components! { Callout => mock::callout },
            "tests/fixtures/components_basic.mdx"
        );
    }

    #[tokio::test]
    async fn renders_component_and_markdown() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components! { Callout => mock::callout },
            "tests/fixtures/components_basic.mdx"
        )
        .expect("view should render successfully");
        let html = view.render(&cx);

        // Should have the heading from markdown (with id attribute).
        assert!(html.contains("<h1 "), "should have <h1>. Got:\n{html}");

        // Should have the paragraph.
        assert!(html.contains("<p>"), "should have <p>. Got:\n{html}");

        // Should have the Callout component output.
        assert!(
            html.contains("mdx-callout"),
            "should have callout component. Got:\n{html}"
        );
        assert!(
            html.contains(r#"data-var="info""#),
            "should have var='info' prop. Got:\n{html}"
        );
    }
}

// --- Backward compatibility: plain .md files (user request) ---

mod md_backward_compat {
    use super::*;

    /// Verify `compile_mdx`! compiles plain .md files (not just .mdx) via
    /// the one-arg form, ensuring the walker handles .md input correctly
    /// after `WalkContext` threading changes.
    #[tokio::test]
    async fn plain_markdown_compiles() {
        let _view = compile_mdx!("tests/fixtures/plain_markdown.md");
    }

    /// Verify plain .md renders headings, paragraphs, and lists.
    #[tokio::test]
    async fn plain_markdown_renders() {
        let view = compile_mdx!("tests/fixtures/plain_markdown.md")
            .expect("view should render successfully");
        let cx = CxTestBuilder::new().build();
        let html = view.render(&cx);

        assert!(html.contains("<h1 "), "should have <h1>. Got:\n{html}");
        assert!(html.contains("<p>"), "should have <p>. Got:\n{html}");
        assert!(html.contains("<ul>"), "should have <ul>. Got:\n{html}");
        assert!(html.contains("<li>"), "should have <li>. Got:\n{html}");
    }

    /// Verify .md files with code blocks and blockquotes compile and render.
    #[tokio::test]
    async fn markdown_with_code_block_compiles() {
        let _view = compile_mdx!("tests/fixtures/mdx_and_markdown.md");
    }

    #[tokio::test]
    async fn markdown_with_code_block_renders() {
        let view = compile_mdx!("tests/fixtures/mdx_and_markdown.md")
            .expect("view should render successfully");
        let cx = CxTestBuilder::new().build();
        let html = view.render(&cx);

        assert!(html.contains("<h2 "), "should have <h2>. Got:\n{html}");
        assert!(html.contains("<pre"), "should have <pre>. Got:\n{html}");
        assert!(
            html.contains("<blockquote>"),
            "should have <blockquote>. Got:\n{html}"
        );
    }
}

// --- Walker-level test: unknown component error propagation ---

mod unknown_component_error {
    use std::path::Path;

    use topcoat_mdx_grammar::walker::WalkContext;
    use topcoat_view_grammar::view::ViewWriter;

    /// Tests that walking MDX content with an unknown `PascalCase` element
    /// pushes an error into ctx.errors. This is walker-level, not
    /// macro-level: the `compile_mdx`! proc-macro is not exercised here.
    #[test]
    fn walker_reports_unknown_component() {
        // Parse MDX with an unregistered component.
        let content = r"Before

<UnknownWidget></UnknownWidget>

After";

        let options = topcoat_mdx_grammar::parse::get_parse_options();
        let root = markdown::to_mdast(content, &options).unwrap();

        let ctx = WalkContext::empty();
        let mut writer = ViewWriter::new();
        if let markdown::mdast::Node::Root(r) = root {
            for child in &r.children {
                topcoat_mdx_grammar::walker::walk_to_writer(&ctx, child, &mut writer);
            }
        }

        // The walker should have pushed an error for the unknown component.
        let errors = ctx.errors.borrow();
        assert!(
            !errors.is_empty(),
            "should have errors for unknown component"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.contains("unknown component 'UnknownWidget'")),
            "should contain 'unknown component' message. Errors: {:?}",
            *errors
        );
    }

    /// Tests that walking a fixture file with an unregistered component
    /// pushes an error. This is walker-level, not macro-level: the
    /// `compile_mdx`! proc-macro is not exercised here.
    #[test]
    fn walker_reports_unknown_component_from_fixture() {
        // Create a temporary .mdx file with an unknown component.
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/components_unknown_test.mdx");

        // Write the fixture if it doesn't exist.
        if !fixture_path.exists() {
            std::fs::write(&fixture_path, "<NotRegistered></NotRegistered>").unwrap();
        }

        // The two-arg form with empty registry should fail to compile.
        // We can't test this at compile time (it would make the test binary
        // not compile), so we test the walker-level error collection instead.
        let content = std::fs::read_to_string(&fixture_path).unwrap();
        let options = topcoat_mdx_grammar::parse::get_parse_options();
        let root = markdown::to_mdast(&content, &options).unwrap();

        let ctx = WalkContext::empty();
        let mut writer = ViewWriter::new();
        if let markdown::mdast::Node::Root(r) = root {
            for child in &r.children {
                topcoat_mdx_grammar::walker::walk_to_writer(&ctx, child, &mut writer);
            }
        }

        let errors = ctx.errors.borrow();
        assert!(
            !errors.is_empty(),
            "should have errors for component not in registry"
        );
    }
}

// ---------------------------------------------------------------------------
// Task 2: Integration test fixtures, comprehensive component coverage
// ---------------------------------------------------------------------------

// --- components_nested: component containing component ---

mod components_nested {
    use super::*;

    #[tokio::test]
    async fn compiles() {
        let cx = CxTestBuilder::new().build();
        let _view = compile_mdx_with_cx!(cx =>
            mdx_components! {
                NestedOuter => mock::nested_outer,
                NestedInner => mock::nested_inner,
            },
            "tests/fixtures/components_nested.mdx"
        );
    }

    #[tokio::test]
    async fn renders_nested_components() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components! {
                NestedOuter => mock::nested_outer,
                NestedInner => mock::nested_inner,
            },
            "tests/fixtures/components_nested.mdx"
        )
        .expect("view should render successfully");
        let html = view.render(&cx);

        assert!(
            html.contains("mdx-nested-outer"),
            "should have outer component. Got:\n{html}"
        );
        assert!(
            html.contains("mdx-nested-inner"),
            "should have inner component. Got:\n{html}"
        );
    }
}

// --- components_self_closing: empty tags and markdown hr ---

mod components_self_closing {
    use super::*;

    #[tokio::test]
    async fn compiles() {
        let cx = CxTestBuilder::new().build();
        let _view = compile_mdx_with_cx!(cx =>
            mdx_components! { Divider => mock::divider },
            "tests/fixtures/components_self_closing.mdx"
        );
    }

    #[tokio::test]
    async fn renders_empty_component_and_html() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components! { Divider => mock::divider },
            "tests/fixtures/components_self_closing.mdx"
        )
        .expect("view should render successfully");
        let html = view.render(&cx);

        // Empty tag pair <Divider></Divider> should produce component output.
        assert!(
            html.contains("mdx-divider"),
            "should have divider component. Got:\n{html}"
        );
        // Markdown horizontal rule (---) should produce <hr>.
        assert!(
            html.contains("<hr>"),
            "should have <hr> from markdown horizontal rule. Got:\n{html}"
        );
    }
}

// --- components_bare_attrs: bare attributes ---

mod components_bare_attrs {
    use super::*;

    #[tokio::test]
    async fn compiles() {
        let cx = CxTestBuilder::new().build();
        let _view = compile_mdx_with_cx!(cx =>
            mdx_components! { BareAttr => mock::bare_attr },
            "tests/fixtures/components_bare_attrs.mdx"
        );
    }

    #[tokio::test]
    async fn renders_bare_attributes() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components! { BareAttr => mock::bare_attr },
            "tests/fixtures/components_bare_attrs.mdx"
        )
        .expect("view should render successfully");
        let html = view.render(&cx);

        // Bare attribute `dismissible` should coerce to true.
        assert!(
            html.contains(r#"data-dismissible="true""#),
            "bare attribute should coerce to true. Got:\n{html}"
        );
        // Explicit "false" should coerce to false.
        assert!(
            html.contains(r#"data-dismissible="false""#),
            "explicit false should stay false. Got:\n{html}"
        );
    }
}

// --- components_prop_types: all coercion types ---

mod components_prop_types {
    use super::*;

    #[tokio::test]
    async fn compiles() {
        let cx = CxTestBuilder::new().build();
        let _view = compile_mdx_with_cx!(cx =>
            mdx_components! { Config => mock::config },
            "tests/fixtures/components_prop_types.mdx"
        );
    }

    #[tokio::test]
    async fn renders_all_prop_types() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components! { Config => mock::config },
            "tests/fixtures/components_prop_types.mdx"
        )
        .expect("view should render successfully");
        let html = view.render(&cx);

        assert!(
            html.contains("mdx-config"),
            "should have config component. Got:\n{html}"
        );
        assert!(
            html.contains(r#"data-enabled="true""#),
            "bool coercion. Got:\n{html}"
        );
        assert!(
            html.contains(r#"data-count="42""#),
            "int coercion. Got:\n{html}"
        );
        assert!(
            html.contains(r#"data-ratio="3.14""#),
            "float coercion. Got:\n{html}"
        );
        assert!(
            html.contains(r#"data-label="hello""#),
            "string coercion. Got:\n{html}"
        );
    }
}

// --- components_mixed_content: markdown + components at same level ---

mod components_mixed_content {
    use super::*;

    #[tokio::test]
    async fn compiles() {
        let cx = CxTestBuilder::new().build();
        let _view = compile_mdx_with_cx!(cx =>
            mdx_components! { Badge => mock::badge },
            "tests/fixtures/components_mixed_content.mdx"
        );
    }

    #[tokio::test]
    async fn renders_mixed_content() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components! { Badge => mock::badge },
            "tests/fixtures/components_mixed_content.mdx"
        )
        .expect("view should render successfully");
        let html = view.render(&cx);

        // Should have markdown heading and paragraphs (heading has id attribute).
        assert!(html.contains("<h1 "), "should have heading. Got:\n{html}");
        assert!(html.contains("<p>"), "should have paragraphs. Got:\n{html}");
        // Should have the Badge component.
        assert!(
            html.contains("mdx-badge"),
            "should have badge component. Got:\n{html}"
        );
    }
}

// --- components_child_content: component wrapping child component ---

mod components_child_content {
    use super::*;

    #[tokio::test]
    async fn compiles() {
        let cx = CxTestBuilder::new().build();
        let _view = compile_mdx_with_cx!(cx =>
            mdx_components! {
                Wrapper => mock::wrapper,
                Badge => mock::badge,
            },
            "tests/fixtures/components_child_content.mdx"
        );
    }

    #[tokio::test]
    async fn renders_child_content() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components! {
                Wrapper => mock::wrapper,
                Badge => mock::badge,
            },
            "tests/fixtures/components_child_content.mdx"
        )
        .expect("view should render successfully");
        let html = view.render(&cx);

        // Wrapper should render its outer section.
        assert!(
            html.contains("mdx-wrapper"),
            "should have wrapper component. Got:\n{html}"
        );
        // Badge should appear as child content inside the wrapper.
        assert!(
            html.contains("mdx-badge"),
            "should have badge child component. Got:\n{html}"
        );
    }
}
