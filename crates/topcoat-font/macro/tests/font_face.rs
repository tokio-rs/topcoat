use topcoat::context::Cx;
use topcoat::font::{FontFace, font_face};

fn render(face: &FontFace) -> String {
    let mut out = String::new();
    face.fmt(&Cx::empty(), &mut out).unwrap();
    out
}

#[test]
fn const_face_with_string_family_uses_const_new() {
    const FACE: FontFace = font_face! {
        font-family: "Inter";
        src: local("Inter"), url("/inter.woff2") format("woff2");
    };
    assert_eq!(
        render(&FACE),
        r#"@font-face { font-family: "Inter"; src: local("Inter"), url("/inter.woff2") format(woff2) }"#,
    );
}

#[test]
fn const_face_with_all_descriptors() {
    const FACE: FontFace = font_face! {
        font-family: "Inter";
        src: url("/inter.woff2") format("woff2") tech("variations");
        font-weight: 400 700;
        font-style: oblique 14deg;
        unicode-range: U+0041-005A, U+0061-007A;
    };
    assert_eq!(
        render(&FACE),
        r#"@font-face { font-family: "Inter"; src: url("/inter.woff2") format(woff2) tech(variations); font-weight: 400 700; font-style: oblique 14deg; unicode-range: U+0041-005A, U+0061-007A }"#,
    );
}

#[test]
fn descriptors_in_any_order() {
    const FACE: FontFace = font_face! {
        unicode-range: U+0041-005A;
        font-style: italic;
        src: url("/inter.woff2") format("woff2");
        font-weight: 700;
        font-family: "Inter";
    };
    assert_eq!(
        render(&FACE),
        r#"@font-face { font-family: "Inter"; src: url("/inter.woff2") format(woff2); font-weight: 700; font-style: italic; unicode-range: U+0041-005A }"#,
    );
}

#[test]
fn bare_oblique_style() {
    const FACE: FontFace = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-style: oblique;
    };
    assert_eq!(
        render(&FACE),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-style: oblique }"#,
    );
}

#[test]
fn normal_style() {
    const FACE: FontFace = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-style: normal;
    };
    assert_eq!(
        render(&FACE),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-style: normal }"#,
    );
}

#[test]
fn oblique_angle_range() {
    const FACE: FontFace = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-style: oblique 20deg 40deg;
    };
    assert_eq!(
        render(&FACE),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-style: oblique 20deg 40deg }"#,
    );
}

#[test]
fn single_weight() {
    const FACE: FontFace = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-weight: 400;
    };
    assert_eq!(
        render(&FACE),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-weight: 400 }"#,
    );
}

#[test]
fn weight_keywords_normalize_to_numbers() {
    const FACE: FontFace = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-weight: normal bold;
    };
    assert_eq!(
        render(&FACE),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-weight: 400 700 }"#,
    );
}

#[test]
fn negative_oblique_angle() {
    const FACE: FontFace = font_face! {
        font-family: "Inter";
        src: local("Inter");
        font-style: oblique -12.5deg;
    };
    assert_eq!(
        render(&FACE),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); font-style: oblique -12.5deg }"#,
    );
}

#[test]
fn url_with_only_tech_hint() {
    const FACE: FontFace = font_face! {
        font-family: "Inter";
        src: url("/inter.woff2") tech("variations");
    };
    assert_eq!(
        render(&FACE),
        r#"@font-face { font-family: "Inter"; src: url("/inter.woff2") tech(variations) }"#,
    );
}

#[test]
fn url_with_no_hints() {
    const FACE: FontFace = font_face! {
        font-family: "Inter";
        src: url("/inter.woff2");
    };
    assert_eq!(
        render(&FACE),
        r#"@font-face { font-family: "Inter"; src: url("/inter.woff2") }"#,
    );
}

#[test]
fn runtime_face_with_unicode_range() {
    // The unicode range slice must be `'static` so a face built outside a
    // `const` and returned by value still borrows it for long enough.
    fn build() -> FontFace {
        font_face! {
            font-family: "Inter";
            src: local("Inter");
            unicode-range: U+0000-00FF, U+0131;
        }
    }
    assert_eq!(
        render(&build()),
        r#"@font-face { font-family: "Inter"; src: local("Inter"); unicode-range: U+0000-00FF, U+0131 }"#,
    );
}

#[test]
fn runtime_expressions_inside_url_and_local() {
    // A `url(...)`/`local(...)` argument may be a runtime expression; the list
    // is then built at run time rather than as a `const`.
    fn build(href: String, installed: String) -> FontFace {
        font_face! {
            font-family: "Inter";
            src: local(installed), url(href) format("woff2");
        }
    }
    assert_eq!(
        render(&build("/inter.woff2".to_owned(), "Inter".to_owned())),
        r#"@font-face { font-family: "Inter"; src: local("Inter"), url("/inter.woff2") format(woff2) }"#,
    );
}

#[test]
fn dynamic_family_falls_back_to_new_constructor() {
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
fn string_family_with_convertible_src_uses_try_into() {
    // A string-literal family with an opaque `src` expression must route to
    // `FontFace::new`, whose `TryInto<FontSources>` bound accepts a `Vec`.
    let src = vec![topcoat::font::FontSource::local("Inter")];
    let face = font_face! {
        font-family: "Inter";
        src: src;
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: local("Inter") }"#,
    );
}

#[test]
fn dynamic_src_with_string_family_still_works() {
    static SRC: &[topcoat::font::FontSource] =
        &[topcoat::font::FontSource::url_str("/inter.woff2", None, None)];
    let src = topcoat::font::FontSources::new(SRC);
    let face = font_face! {
        font-family: "Inter";
        src: src;
    };
    assert_eq!(
        render(&face),
        r#"@font-face { font-family: "Inter"; src: url("/inter.woff2") }"#,
    );
}
