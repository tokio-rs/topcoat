use topcoat::{context::CxTestBuilder, mdx::compile_mdx};

#[tokio::test]
async fn compile_mdx_with_frontmatter_compiles() {
    // compile_mdx! on a frontmatter file emits a YAML const + view tokens.
    let view = compile_mdx!("tests/fixtures/frontmatter_basic.mdx")
        .expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);
    // The frontmatter should NOT appear in the rendered HTML.
    assert!(
        !html.contains("---"),
        "frontmatter should not render as content"
    );
    // But the body content should render.
    assert!(html.contains("Hello from MDX"), "body should render");
}

// ---- Backward compatibility: compile_mdx! still works ----

#[tokio::test]
async fn compile_mdx_backward_compat_one_arg() {
    // Verify one-arg compile_mdx! still compiles with plain markdown fixture.
    let view = compile_mdx!("tests/fixtures/tracer.mdx").expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);
    assert!(html.contains("Tracer Test"), "tracer content should render");
}

// ---- No frontmatter files ----

#[tokio::test]
async fn compile_mdx_without_frontmatter() {
    // Verify compile_mdx! on a no-frontmatter file compiles (no YAML const emitted).
    let view = compile_mdx!("tests/fixtures/frontmatter_empty.mdx")
        .expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);
    assert!(
        html.contains("No Frontmatter"),
        "plain content should render"
    );
}

// ---- Complex frontmatter ----

#[tokio::test]
async fn compile_mdx_complex_frontmatter() {
    // Verify compile_mdx! on a complex frontmatter file compiles.
    let view = compile_mdx!("tests/fixtures/frontmatter_complex.mdx")
        .expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);
    assert!(
        html.contains("Complex Frontmatter Test"),
        "body should render"
    );
    assert!(
        !html.contains("---"),
        "frontmatter should not render as content"
    );
}

// ---- .md file with frontmatter ----

#[tokio::test]
async fn compile_mdx_handles_md_extension_with_frontmatter() {
    let view =
        compile_mdx!("tests/fixtures/frontmatter_md.md").expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);
    assert!(html.contains("<h1 "), "body should render");
    assert!(
        !html.contains("---"),
        "frontmatter should not render as content"
    );
}

// ---- Custom fields: rendering should work regardless of unknown fields ----

#[tokio::test]
async fn compile_mdx_blog_post_with_custom_fields() {
    // Blog post has non-standard fields: subtitle, publishDate,
    // lastModifiedDate, keywords. Body should still render.
    let view = compile_mdx!("tests/fixtures/frontmatter_blog_post.mdx")
        .expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);
    assert!(
        !html.contains("---"),
        "frontmatter should not render as content"
    );
    assert!(
        html.contains("Blog Post with Custom Metadata"),
        "heading should render"
    );
    assert!(html.contains("First list item"), "list item should render");
}

#[tokio::test]
async fn compile_mdx_arbitrary_custom_fields() {
    // Custom fields (category, author, custom_key) don't break rendering.
    let view = compile_mdx!("tests/fixtures/frontmatter_minimal_custom.mdx")
        .expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);
    assert!(
        !html.contains("---"),
        "frontmatter should not render as content"
    );
    assert!(html.contains("Custom Fields Test"), "heading should render");
}

#[tokio::test]
async fn compile_mdx_toml_custom_fields() {
    // TOML frontmatter with custom fields (subtitle, my_field, nested).
    let view = compile_mdx!("tests/fixtures/frontmatter_toml_custom.mdx")
        .expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);
    assert!(
        !html.contains("+++"),
        "TOML frontmatter should not render as content"
    );
    assert!(html.contains("TOML Custom Fields"), "heading should render");
}
