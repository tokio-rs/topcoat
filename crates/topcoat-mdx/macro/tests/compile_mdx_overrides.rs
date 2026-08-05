#![allow(clippy::approx_constant)]

use topcoat::{context::CxTestBuilder, mdx::compile_mdx, view as topcoat_view_module};
use topcoat_view_module::{View, component, view};

type Result<T = View> = topcoat::Result<T>;

/// Helper to run `compile_mdx`! with a `__cx` binding so that component
/// code (`__view(__cx, ...)`) compiles correctly.
macro_rules! compile_mdx_with_cx {
    ( $cx:expr => $( $arg:tt )* ) => {{
        let __cx = &$cx;
        compile_mdx!($( $arg )*)
    }};
}

// ---------------------------------------------------------------------------
// Mock components for override integration tests
// ---------------------------------------------------------------------------

mod mock {
    use super::*;

    /// Custom link component that accepts `href` prop (matching HTML <a> attr).
    #[component]
    pub async fn custom_link(href: &'static str, #[default] child: View) -> Result {
        view! { <a class="custom-link" href=(href)>(child)</a> }
    }
}

// ---------------------------------------------------------------------------
// Override integration tests
// ---------------------------------------------------------------------------

mod overrides_link {
    use super::*;

    /// Verify that `compile_mdx!` with an `overrides` arg compiles.
    #[tokio::test]
    async fn compiles_with_overrides() {
        let cx = CxTestBuilder::new().build();
        let _view = compile_mdx_with_cx!(cx =>
            mdx_components!{},
            overrides = { "a" => mock::custom_link },
            "tests/fixtures/overrides_link.mdx"
        );
    }

    /// Verify that links render through the override component.
    #[tokio::test]
    async fn renders_link_through_override() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components!{},
            overrides = { "a" => mock::custom_link },
            "tests/fixtures/overrides_link.mdx"
        )
        .expect("view should render successfully");
        let html = view.render(&cx);

        assert!(
            html.contains("custom-link"),
            "link should render through override component. Got:\n{html}"
        );
    }
}

mod overrides_xss_safety {
    use super::*;

    /// Verify that javascript: URLs are NOT routed through the override
    /// component even when one is registered, as XSS protection.
    #[tokio::test]
    async fn dangerous_url_not_overridden() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components!{},
            overrides = { "a" => mock::custom_link },
            "tests/fixtures/overrides_xss.mdx"
        )
        .expect("view should render safely");
        let html = view.render(&cx);

        assert!(
            !html.contains("custom-link"),
            "javascript: URL should NOT route through override. Got:\n{html}"
        );
    }
}

// ---------------------------------------------------------------------------
// Mock components for expanded override + wrapper tests
// ---------------------------------------------------------------------------

#[allow(unused_variables)]
// Mock components accept props the MDX walker passes (`id`, `data_lang`, ...) but
// only use `child` in their view bodies. The module-level allow suppresses the
// resulting unused-variable warnings.
mod mock_all {
    use super::*;

    /// Heading override component that adds an anchor class.
    #[component]
    pub async fn heading(id: &'static str, #[default] child: View) -> Result {
        view! { <div class="heading-override">(child)</div> }
    }

    /// Image override component that wraps <img> in a figure.
    #[component]
    pub async fn picture(src: &'static str, alt: &'static str, #[default] child: View) -> Result {
        view! {
            <figure class="picture-override">
                <img src=(src) alt=(alt) />
                (child)
            </figure>
        }
    }

    /// Code block override component.
    #[component]
    pub async fn code_block(data_lang: &'static str, #[default] child: View) -> Result {
        view! { <div class="code-block-override">(child)</div> }
    }

    /// Thematic break override component.
    #[component]
    pub async fn separator(#[default] child: View) -> Result {
        view! { <div class="separator-override">(child)</div> }
    }

    /// Wrapper component that receives child: View.
    #[component]
    pub async fn article_wrapper(#[default] child: View) -> Result {
        view! { <article class="wrapped-article">(child)</article> }
    }
}

// ---------------------------------------------------------------------------
// Expanded override integration tests
// ---------------------------------------------------------------------------

mod overrides_all_elements {
    use super::*;

    /// Verify that heading, image, code block, and hr overrides compile
    /// together through `compile_mdx!`.
    #[tokio::test]
    async fn compiles_with_multiple_overrides() {
        let cx = CxTestBuilder::new().build();
        let _view = compile_mdx_with_cx!(cx =>
            mdx_components!{},
            overrides = {
                "h1" => mock_all::heading,
                "h2" => mock_all::heading,
                "img" => mock_all::picture,
                "pre" => mock_all::code_block,
                "hr" => mock_all::separator,
            },
            "tests/fixtures/overrides_all.mdx"
        );
    }

    /// Verify heading override renders through the component.
    #[tokio::test]
    async fn renders_heading_through_override() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components!{},
            overrides = {
                "h1" => mock_all::heading,
            },
            "tests/fixtures/overrides_all.mdx"
        )
        .expect("view should render");
        let html = view.render(&cx);

        assert!(
            html.contains("heading-override"),
            "heading should render through override. Got:\n{html}"
        );
    }

    /// Verify image override renders through the component.
    #[tokio::test]
    async fn renders_image_through_override() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components!{},
            overrides = { "img" => mock_all::picture },
            "tests/fixtures/overrides_all.mdx"
        )
        .expect("view should render");
        let html = view.render(&cx);

        assert!(
            html.contains("picture-override"),
            "image should render through override. Got:\n{html}"
        );
    }

    /// Verify code block override renders through the component.
    #[tokio::test]
    async fn renders_code_block_through_override() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components!{},
            overrides = { "pre" => mock_all::code_block },
            "tests/fixtures/overrides_all.mdx"
        )
        .expect("view should render");
        let html = view.render(&cx);

        assert!(
            html.contains("code-block-override"),
            "code block should render through override. Got:\n{html}"
        );
    }

    /// Verify thematic break override renders through the component.
    #[tokio::test]
    async fn renders_hr_through_override() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components!{},
            overrides = { "hr" => mock_all::separator },
            "tests/fixtures/overrides_all.mdx"
        )
        .expect("view should render");
        let html = view.render(&cx);

        assert!(
            html.contains("separator-override"),
            "thematic break should render through override. Got:\n{html}"
        );
    }
}

// ---------------------------------------------------------------------------
// Wrapper integration tests
// ---------------------------------------------------------------------------

mod wrapper {
    use super::*;

    /// Verify that `mdx_page!` with a `wrapper` arg compiles and the
    /// wrapper component wraps the MDX content.
    #[tokio::test]
    async fn mdx_page_with_wrapper() {
        use topcoat::mdx::mdx_page;

        mdx_page!(
            "/wrapper-test",
            "tests/fixtures/wrapper_test.mdx",
            wrapper = mock_all::article_wrapper
        );

        // The page was registered via inventory; we can't easily call it
        // here without a router, but the compilation itself proves the
        // wrapper tokens were generated correctly.
    }

    /// Verify that `compile_mdx!` with a wrapper arg compiles.
    #[tokio::test]
    async fn compile_mdx_with_wrapper() {
        let cx = CxTestBuilder::new().build();
        let _view = compile_mdx_with_cx!(cx =>
            mdx_components!{},
            wrapper = mock_all::article_wrapper,
            "tests/fixtures/wrapper_test.mdx"
        );
    }

    /// Verify wrapper renders around the content.
    #[tokio::test]
    async fn wrapper_renders_around_content() {
        let cx = CxTestBuilder::new().build();
        let view = compile_mdx_with_cx!(cx =>
            mdx_components!{},
            wrapper = mock_all::article_wrapper,
            "tests/fixtures/wrapper_test.mdx"
        )
        .expect("view should render");
        let html = view.render(&cx);

        assert!(
            html.contains("wrapped-article"),
            "wrapper component should render around content. Got:\n{html}"
        );
    }
}

// ---------------------------------------------------------------------------
// mdx_page! with components wiring test
// ---------------------------------------------------------------------------

mod mdx_page_components {
    use topcoat::mdx::mdx_page;

    use super::*;

    #[component]
    async fn callout_with_var(var: &'static str, #[default] child: View) -> Result {
        view! { <div class=(var)>(child)</div> }
    }

    /// Verify that `mdx_page!` with `components = {...}` compiles and passes
    /// the component registry through to the walker.
    #[test]
    fn mdx_page_with_components_compiles() {
        mdx_page!(
            "/components-wiring-test",
            "tests/fixtures/components_basic.mdx",
            components = { Callout => callout_with_var }
        );
        // Compilation proves components were threaded through compile_mdx_file.
    }
}
