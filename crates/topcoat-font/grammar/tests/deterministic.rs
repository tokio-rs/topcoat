use topcoat_core_grammar::testing::assert_deterministic;
use topcoat_font_grammar::{font::Font, font_face::FontFace};

#[test]
fn font_face() {
    assert_deterministic(|| {
        syn::parse_str::<FontFace>(
            r#"
            unicode-range: U+0041-005A, U+0061-007A;
            font-family: "Inter";
            src: url("/inter.woff2") format("woff2") tech("variations"), local("Inter");
            font-weight: 400 700;
            font-style: oblique 14deg;
            font-display: swap;
            "#,
        )
    });
}

#[test]
fn font() {
    assert_deterministic(|| {
        syn::parse_str::<Font>(
            r#"
            "Inter",
            @font-face {
                src: url("/inter-400.woff2") format("woff2");
                font-weight: 400;
            }
            @font-face {
                src: url("/inter-700.woff2") format("woff2");
                font-weight: 700;
                font-style: italic;
            }
            "#,
        )
    });
}

#[cfg(feature = "fontsource")]
#[test]
fn fontsource_font_face() {
    use topcoat_font_grammar::fontsource::font_face::FontsourceFontFace;

    assert_deterministic(|| {
        syn::parse_str::<FontsourceFontFace>("ROBOTO, weight: 400, style: Normal")
    });
}

#[cfg(feature = "fontsource")]
#[test]
fn fontsource_font() {
    use topcoat_font_grammar::fontsource::font::FontsourceFont;

    assert_deterministic(|| syn::parse_str::<FontsourceFont>("ROBOTO"));
    assert_deterministic(|| syn::parse_str::<FontsourceFont>("ROBOTO, weight: 400, style: Normal"));
}
