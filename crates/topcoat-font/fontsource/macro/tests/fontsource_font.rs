use topcoat::context::Cx;
use topcoat::font::Font;
use topcoat::font::fontsource::fontsource_font;

/// Roboto's vendored `unicode-range` for the `latin` subset.
const LATIN: &str = "U+0000-00FF, U+0131, U+0152-0153, U+02BB-02BC, U+02C6, U+02DA, U+02DC, U+0304, U+0308, U+0329, U+2000-206F, U+20AC, U+2122, U+2191, U+2193, U+2212, U+2215, U+FEFF, U+FFFD";

fn render(font: Font) -> String {
    let mut out = String::new();
    font.faces().fmt(&Cx::default(), &mut out).unwrap();
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
fn bare_family_expands_every_weight_and_style() {
    // Roboto ships nine weights and two styles, crossed over its single default
    // subset.
    let font = fontsource_font!(ROBOTO);
    assert_eq!(font.family(), "Roboto");
    assert_eq!(font.faces().len(), 9 * 2);
}

#[test]
fn omitting_the_subset_uses_the_default_only() {
    // Roboto ships nine subsets, but leaving `subset` off pulls in the default
    // (`latin`) alone rather than all of them.
    let font = fontsource_font!(ROBOTO, weight: 400, style: Normal);
    assert_eq!(
        render(font),
        roboto("latin-400-normal", 400, "normal", LATIN)
    );
}

#[test]
fn a_single_face() {
    let font = fontsource_font!(ROBOTO, weight: 400, style: Normal, subset: Latin);
    assert_eq!(font.faces().len(), 1);
    assert_eq!(
        render(font),
        roboto("latin-400-normal", 400, "normal", LATIN)
    );
}

#[test]
fn lists_cross_product_into_faces() {
    let font = fontsource_font!(ROBOTO, weight: [400, 700], style: Normal, subset: Latin);
    let faces = [
        roboto("latin-400-normal", 400, "normal", LATIN),
        roboto("latin-700-normal", 700, "normal", LATIN),
    ];
    assert_eq!(render(font), faces.join(" "));
}

#[test]
fn every_axis_multiplies() {
    // Two weights * two styles * one subset.
    let font = fontsource_font!(
        ROBOTO,
        weight: [400, 700],
        style: [Normal, Italic],
        subset: Latin,
    );
    assert_eq!(font.faces().len(), 4);
}

#[test]
fn display_applies_to_every_face() {
    let font = fontsource_font!(
        ROBOTO,
        weight: [400, 700],
        style: Normal,
        subset: Latin,
        display: Optional,
    );
    let faces = [
        roboto_display("latin-400-normal", 400, "normal", "optional", LATIN),
        roboto_display("latin-700-normal", 700, "normal", "optional", LATIN),
    ];
    assert_eq!(render(font), faces.join(" "));
}

#[test]
fn self_hosting_changes_the_sources() {
    let cdn = fontsource_font!(ROBOTO, weight: 400, style: Normal, subset: Latin);
    let asset = fontsource_font!(
        ROBOTO,
        weight: 400,
        style: Normal,
        subset: Latin,
        host: Asset,
    );
    assert_ne!(cdn.faces(), asset.faces());
}
