use topcoat::context::Cx;
use topcoat::font::{FontFace, FontSource, font_face};

fn render(face: &FontFace) -> String {
    let mut out = String::new();
    face.fmt(&Cx::empty(), &mut out).unwrap();
    out
}

#[test]
fn minimal_face() {
    let face = font_face! {
        font-family: "Inter";
        src: local("Inter");
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter") }"#,
    );
}

#[test]
fn all_descriptors() {
    let face = font_face! {
        font-family: "Inter";
        src: url("/inter.woff2") format("woff2") tech("variations");
        font-weight: 400 700;
        font-style: oblique 14deg;
        font-display: swap;
        unicode-range: U+0041-005A, U+0061-007A;
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: url("/inter.woff2") format(woff2) tech(variations); font-weight: 400 700; font-style: oblique 14deg; font-display: swap; unicode-range: U+0041-005A, U+0061-007A }"#,
    );
}

#[test]
fn font_display_keyword() {
    let face = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-display: optional;
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-display: optional }"#,
    );
}

#[test]
fn descriptors_in_any_order() {
    let face = font_face! {
        unicode-range: U+0041-005A;
        font-style: italic;
        src: url("/inter.woff2") format("woff2");
        font-weight: 700;
        font-family: "Inter";
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: url("/inter.woff2") format(woff2); font-weight: 700; font-style: italic; unicode-range: U+0041-005A }"#,
    );
}

#[test]
fn multiple_sources() {
    let face = font_face! {
        font-family: "Inter";
        src: local("Inter"), url("/inter.woff2") format("woff2");
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter"), url("/inter.woff2") format(woff2) }"#,
    );
}

#[test]
fn url_with_only_tech_hint() {
    let face = font_face! {
        font-family: "Inter";
        src: url("/inter.woff2") tech("variations");
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: url("/inter.woff2") tech(variations) }"#,
    );
}

#[test]
fn url_with_no_hints() {
    let face = font_face! {
        font-family: "Inter";
        src: url("/inter.woff2");
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: url("/inter.woff2") }"#,
    );
}

#[test]
fn single_weight() {
    let face = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-weight: 400;
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-weight: 400 }"#,
    );
}

#[test]
fn weight_keywords_normalize_to_numbers() {
    let face = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-weight: normal bold;
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-weight: 400 700 }"#,
    );
}

#[test]
fn bare_oblique_style() {
    let face = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-style: oblique;
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-style: oblique }"#,
    );
}

#[test]
fn normal_style() {
    let face = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-style: normal;
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-style: normal }"#,
    );
}

#[test]
fn oblique_angle_range() {
    let face = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-style: oblique 20deg 40deg;
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-style: oblique 20deg 40deg }"#,
    );
}

#[test]
fn negative_oblique_angle() {
    let face = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-style: oblique -12.5deg;
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-style: oblique -12.5deg }"#,
    );
}

#[test]
fn runtime_expressions_inside_url_and_local() {
    let href = String::from("/inter.woff2");
    let installed = String::from("Inter");
    let face = font_face! {
        font-family: "Inter";
        src: local(installed), url(href) format("woff2");
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter"), url("/inter.woff2") format(woff2) }"#,
    );
}

#[test]
fn dynamic_family() {
    let family = String::from("Inter");
    let face = font_face! {
        font-family: family;
        src: local("Inter");
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter") }"#,
    );
}

#[test]
fn src_from_expression() {
    let src = vec![FontSource::local("Inter")];
    let face = font_face! {
        font-family: "Inter";
        src: src;
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter") }"#,
    );
}
