use topcoat::context::Cx;
use topcoat::font::FontFace;
use topcoat::font::fontsource::fontsource_font_face;

/// Roboto's vendored `unicode-range` for the `latin` subset.
const LATIN: &str = "U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD";

/// Roboto's vendored `unicode-range` for the `cyrillic` subset.
const CYRILLIC: &str = "U+0301, U+0400-045F, U+0490-0491, U+04B0-04B1, U+2116";

fn render(face: &FontFace) -> String {
    let mut out = String::new();
    face.fmt(&Cx::empty(), &mut out).unwrap();
    out
}

/// Builds the `@font-face` rule a Roboto CDN face renders to, using the default
/// `swap` display strategy.
fn roboto(file: &str, weight: u16, style: &str, range: &str) -> String {
    roboto_display(file, weight, style, "swap", range)
}

/// Builds the `@font-face` rule a Roboto CDN face renders to with an explicit
/// `font-display` strategy.
fn roboto_display(file: &str, weight: u16, style: &str, display: &str, range: &str) -> String {
    format!(
        r#"@font-face {{ font-family: "Roboto"; src: url("https://cdn.jsdelivr.net/fontsource/fonts/roboto@latest/{file}.woff2") format(woff2); font-weight: {weight}; font-style: {style}; font-display: {display}; unicode-range: {range} }}"#
    )
}

#[test]
fn minimal_face_uses_the_default_subset() {
    let face = fontsource_font_face!("Roboto", weight: 400, style: Normal);
    assert_eq!(
        render(&face),
        roboto("latin-400-normal", 400, "normal", LATIN)
    );
}

#[test]
fn italic_style() {
    let face = fontsource_font_face!("Roboto", weight: 400, style: Italic);
    assert_eq!(
        render(&face),
        roboto("latin-400-italic", 400, "italic", LATIN)
    );
}

#[test]
fn explicit_subset_drives_the_url_and_unicode_range() {
    let face = fontsource_font_face!("Roboto", weight: 700, style: Italic, subset: Cyrillic);
    assert_eq!(
        render(&face),
        roboto("cyrillic-700-italic", 700, "italic", CYRILLIC),
    );
}

#[test]
fn arguments_in_any_order() {
    let face = fontsource_font_face!("Roboto", style: Normal, subset: Cyrillic, weight: 500);
    assert_eq!(
        render(&face),
        roboto("cyrillic-500-normal", 500, "normal", CYRILLIC)
    );
}

#[test]
fn display_defaults_to_swap() {
    let face = fontsource_font_face!("Roboto", weight: 400, style: Normal);
    assert!(render(&face).contains("font-display: swap"));
}

#[test]
fn explicit_display_overrides_the_default() {
    let face = fontsource_font_face!("Roboto", weight: 400, style: Normal, display: Fallback);
    assert_eq!(
        render(&face),
        roboto_display("latin-400-normal", 400, "normal", "fallback", LATIN),
    );
}

#[test]
fn host_defaults_to_jsdelivr() {
    let default = fontsource_font_face!("Roboto", weight: 400, style: Normal);
    let explicit = fontsource_font_face!("Roboto", weight: 400, style: Normal, host: JsDelivr);
    assert_eq!(default, explicit);
}

#[test]
fn self_hosting_changes_the_source() {
    let cdn = fontsource_font_face!("Roboto", weight: 400, style: Normal);
    let asset = fontsource_font_face!("Roboto", weight: 400, style: Normal, host: Asset);
    // The CDN face points at jsDelivr; the asset face is served from our own
    // bundle, so the two sources differ.
    assert_ne!(cdn, asset);
}
