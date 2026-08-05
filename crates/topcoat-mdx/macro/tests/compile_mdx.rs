#![allow(clippy::approx_constant)]

use topcoat::{context::CxTestBuilder, mdx::compile_mdx};

// ---- Tracer fixture ----

#[tokio::test]
async fn tracer_compiles() {
    // Verify the macro expands and compiles without errors.
    let _view = compile_mdx!("tests/fixtures/tracer.mdx");
}

#[tokio::test]
async fn tracer_renders() {
    // Verifies the tracer fixture compiles and renders mixed markdown content.
    // Raw HTML passthrough is disabled; the fixture uses pure markdown.
    let view = compile_mdx!("tests/fixtures/tracer.mdx").expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);

    // Should render the heading and paragraph.
    assert!(
        html.contains("Tracer Test"),
        "should have h1 text. Got:\n{html}"
    );
    assert!(
        html.contains("<strong>bold</strong>"),
        "should have bold. Got:\n{html}"
    );
    assert!(
        html.contains("<em>italic</em>"),
        "should have italic. Got:\n{html}"
    );
    // Blockquote replaced the old raw HTML div.
    assert!(
        html.contains("<blockquote>"),
        "should have blockquote. Got:\n{html}"
    );
}

// ---- CommonMark fixture ----

#[tokio::test]
async fn commonmark_compiles() {
    // Verify the macro expands and compiles without errors.
    let _view = compile_mdx!("tests/fixtures/commonmark.mdx");
}

#[tokio::test]
async fn commonmark_renders() {
    let view =
        compile_mdx!("tests/fixtures/commonmark.mdx").expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);

    // Heading levels 1-6 (id attributes added by heading ID generation)
    assert!(html.contains("<h1 "), "should have <h1>. Got:\n{html}");
    assert!(html.contains("<h2 "), "should have <h2>");
    assert!(html.contains("<h3 "), "should have <h3>");
    assert!(html.contains("<h4 "), "should have <h4>");
    assert!(html.contains("<h5 "), "should have <h5>");
    assert!(html.contains("<h6 "), "should have <h6>");

    // Paragraph
    assert!(html.contains("<p>"), "should have <p>");

    // Inline formatting
    assert!(html.contains("<strong>"), "should have <strong>");
    assert!(html.contains("<em>"), "should have <em>");

    // Link with correct href value
    assert!(
        html.contains(r#"href="https://example.com""#),
        "should have correct href value. Got:\n{html}"
    );

    // Image with src and alt attribute values
    assert!(
        html.contains(r#"src="photo.png""#),
        "should have correct image src value. Got:\n{html}"
    );
    assert!(
        html.contains(r#"alt="Image alt""#),
        "should have correct alt value. Got:\n{html}"
    );

    // Code block: <pre> with optional data-* attributes, then <code
    assert!(
        html.contains("<pre") && html.contains("<code"),
        "should have <pre> and <code. Got:\n{html}"
    );

    // Blockquote
    assert!(
        html.contains("<blockquote>"),
        "should have <blockquote>. Got:\n{html}"
    );

    // Lists
    assert!(html.contains("<ul>"), "should have <ul>");
    assert!(html.contains("<li>"), "should have <li>");
    assert!(html.contains("<ol>"), "should have <ol>");

    // Thematic break
    assert!(html.contains("<hr>"), "should have <hr>");

    // Hard break
    assert!(html.contains("<br>"), "should have <br>");

    // Inline code
    assert!(html.contains("<code>"), "should have inline <code>");
}

// ---- GFM fixture ----

#[tokio::test]
async fn gfm_compiles() {
    let _view = compile_mdx!("tests/fixtures/gfm.mdx");
}

#[tokio::test]
async fn gfm_renders() {
    let view = compile_mdx!("tests/fixtures/gfm.mdx").expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);

    // Table structure
    assert!(
        html.contains("<table>"),
        "should have <table>. Got:\n{html}"
    );
    assert!(html.contains("<thead>"), "should have <thead>");
    assert!(html.contains("<tbody>"), "should have <tbody>");
    assert!(html.contains("<tr>"), "should have <tr>");
    assert!(html.contains("<th>"), "should have <th>");
    assert!(html.contains("<td>"), "should have <td>");

    // Table alignment values
    assert!(
        html.contains("text-align: left"),
        "should have left alignment. Got:\n{html}"
    );
    assert!(
        html.contains("text-align: right"),
        "should have right alignment. Got:\n{html}"
    );
    assert!(
        html.contains("text-align: center"),
        "should have center alignment. Got:\n{html}"
    );

    // Strikethrough
    assert!(
        html.contains("<del>"),
        "should have <del> for strikethrough. Got:\n{html}"
    );

    // Task list: checkbox input
    assert!(
        html.contains(r#"type="checkbox""#),
        "should have type=\"checkbox\". Got:\n{html}"
    );
}

// ---- Raw HTML fixture ----
// Raw HTML passthrough is disabled. These tests verify
// that raw HTML blocks are dropped by the parser rather than passed through.

#[tokio::test]
async fn raw_html_compiles() {
    let _view = compile_mdx!("tests/fixtures/raw_html.mdx");
}

#[tokio::test]
async fn raw_html_dropped_when_disabled() {
    let view =
        compile_mdx!("tests/fixtures/raw_html.mdx").expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);

    // Raw HTML blocks should NOT appear when passthrough is disabled.
    assert!(
        !html.contains(r#"<div class="test">"#),
        "raw HTML div should be dropped when passthrough is disabled. Got:\n{html}"
    );
    assert!(
        !html.contains(r#"<table class="raw-table">"#),
        "raw HTML table should be dropped when passthrough is disabled. Got:\n{html}"
    );
    // Markdown content between the HTML blocks should still render.
    assert!(
        html.contains("This is a regular paragraph"),
        "markdown paragraphs should still render. Got:\n{html}"
    );
}

// ---- Reference links and footnotes ----

#[tokio::test]
async fn compile_mdx_resolves_reference_links() {
    // Definitions are collected before the walk, so a reference-style link
    // resolves to the declared URL and title.
    let view = compile_mdx!("tests/fixtures/references_footnotes.mdx")
        .expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);

    assert!(
        html.contains(r#"href="https://docs.rs/topcoat""#),
        "reference link should resolve to the definition URL. Got:\n{html}"
    );
    assert!(
        html.contains(r#"title="Topcoat on docs.rs""#),
        "reference link should carry the definition title. Got:\n{html}"
    );
}

#[tokio::test]
async fn compile_mdx_renders_footnote_section() {
    // Footnote definitions render as a section at the end of the document,
    // with anchors that resolve in both directions.
    let view = compile_mdx!("tests/fixtures/references_footnotes.mdx")
        .expect("view should render successfully");
    let cx = CxTestBuilder::new().build();
    let html = view.render(&cx);

    assert!(
        html.contains("The footnote body."),
        "footnote section should render at the document end. Got:\n{html}"
    );
    assert!(
        html.contains(r#"id="fnref-note""#) && html.contains(r##"href="#fn-note""##),
        "reference should link to the footnote. Got:\n{html}"
    );
    assert!(
        html.contains(r#"id="fn-note""#) && html.contains(r##"href="#fnref-note""##),
        "footnote should link back to the reference. Got:\n{html}"
    );
}
