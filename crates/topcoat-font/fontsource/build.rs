use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    env,
    fmt::Write as _,
    fs,
    path::Path,
};

use heck::{ToPascalCase, ToShoutySnakeCase};
use serde::Deserialize;

/// The vendored catalog: an interned `ranges` table and the families that
/// reference it by index.
#[derive(Deserialize)]
struct Catalog {
    ranges: Vec<String>,
    fonts: Vec<FamilyMetadata>,
}

#[derive(Deserialize)]
struct FamilyMetadata {
    id: String,
    family: String,
    subsets: Vec<String>,
    weights: Vec<u16>,
    styles: Vec<String>,
    #[serde(rename = "defSubset")]
    def_subset: String,
    variable: bool,
    category: String,
    license: String,
    #[serde(rename = "type")]
    provider: String,
    /// Maps each named subset to an index into [`Catalog::ranges`].
    #[serde(rename = "unicodeRange", default)]
    unicode_range: BTreeMap<String, usize>,
}

fn main() {
    println!("cargo::rerun-if-changed=fonts.json");

    let json = fs::read_to_string("fonts.json").expect("read fonts.json");
    let parsed: Catalog = serde_json::from_str(&json).expect("parse fonts.json");
    let ranges = parsed.ranges;
    let mut families = parsed.fonts;
    families.sort_by(|a, b| a.id.cmp(&b.id));

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    fs::write(Path::new(&out_dir).join("subset.rs"), subsets(&families)).expect("write subset.rs");
    fs::write(
        Path::new(&out_dir).join("families.rs"),
        catalog(&families, &ranges),
    )
    .expect("write families.rs");
}

/// `PascalCase` enum variant for a subset id (`"latin-ext"` -> `LatinExt`).
fn subset_variant(id: &str) -> String {
    id.to_pascal_case()
}

/// `SCREAMING_SNAKE_CASE` constant name for a family id, prefixed with `_` when
/// it would otherwise start with a digit (`"42dot-sans"` -> `_42DOT_SANS`).
fn family_ident(id: &str) -> String {
    let mut ident = id.to_shouty_snake_case();
    if ident.starts_with(|c: char| c.is_ascii_digit()) {
        ident.insert(0, '_');
    }
    ident
}

fn style_variant(style: &str) -> &'static str {
    match style {
        "normal" => "Normal",
        "italic" => "Italic",
        other => panic!("unknown font style: {other}"),
    }
}

fn subsets(families: &[FamilyMetadata]) -> String {
    let ids: BTreeSet<&str> = families
        .iter()
        .flat_map(|f| f.subsets.iter().map(String::as_str))
        .collect();

    let mut variants = Vec::with_capacity(ids.len());
    let mut seen = HashSet::new();
    for id in ids {
        let variant = subset_variant(id);
        assert!(
            seen.insert(variant.clone()),
            "subset variant collision on `{variant}`"
        );
        variants.push((variant, id));
    }

    let mut out = String::new();
    out.push_str("/// A character subset a font family can ship.\n///\n");
    out.push_str("/// One variant is generated per distinct subset in the vendored Fontsource\n");
    out.push_str("/// catalog.\n");
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]\n");
    out.push_str("#[non_exhaustive]\n");
    out.push_str("pub enum Subset {\n");
    for (variant, id) in &variants {
        writeln!(out, "    /// The `{id}` subset.\n    {variant},").unwrap();
    }
    out.push_str("}\n\nimpl Subset {\n");

    out.push_str("    /// The Fontsource subset id, e.g. `\"latin-ext\"`.\n");
    out.push_str(
        "    #[must_use]\n    pub const fn as_str(self) -> &'static str {\n        match self {\n",
    );
    for (variant, id) in &variants {
        writeln!(out, "            Self::{variant} => {id:?},").unwrap();
    }
    out.push_str("        }\n    }\n\n");

    out.push_str("    /// Parse a Fontsource subset id, returning `None` if it is not in the\n");
    out.push_str("    /// vendored catalog.\n");
    out.push_str("    #[must_use]\n    pub fn from_id(id: &str) -> Option<Self> {\n        Some(match id {\n");
    for (variant, id) in &variants {
        writeln!(out, "            {id:?} => Self::{variant},").unwrap();
    }
    out.push_str("            _ => return None,\n        })\n    }\n\n");

    out.push_str("    /// Parse a variant name, e.g. `\"LatinExt\"`, returning `None` if it is\n");
    out.push_str("    /// not in the vendored catalog.\n");
    out.push_str("    #[must_use]\n    pub fn from_variant(name: &str) -> Option<Self> {\n        Some(match name {\n");
    for (variant, _) in &variants {
        writeln!(out, "            {variant:?} => Self::{variant},").unwrap();
    }
    out.push_str("            _ => return None,\n        })\n    }\n}\n");
    out
}

/// Parses a single `unicode-range` token (`U+0041`, `U+0041-005A`, or a
/// wildcard like `U+30??`) into an inclusive `(start, end)` code point pair.
fn parse_range_token(token: &str) -> (u32, u32) {
    let body = token
        .trim()
        .strip_prefix("U+")
        .or_else(|| token.trim().strip_prefix("u+"))
        .unwrap_or_else(|| panic!("unicode range token missing `U+`: {token:?}"));
    if let Some((start, end)) = body.split_once('-') {
        (parse_hex(start), parse_hex(end))
    } else if body.contains('?') {
        (
            parse_hex(&body.replace('?', "0")),
            parse_hex(&body.replace('?', "F")),
        )
    } else {
        let cp = parse_hex(body);
        (cp, cp)
    }
}

fn parse_hex(s: &str) -> u32 {
    u32::from_str_radix(s.trim(), 16).unwrap_or_else(|_| panic!("invalid unicode hex: {s:?}"))
}

/// Emits one `const UR{n}: UnicodeRanges` per distinct interned range spec.
fn unicode_range_consts(ranges: &[String]) -> String {
    let mut out = String::new();
    for (i, spec) in ranges.iter().enumerate() {
        let entries = spec
            .split(',')
            .filter(|token| !token.trim().is_empty())
            .map(parse_range_token)
            .map(|(start, end)| format!("UnicodeRange::from_u32({start:#x}, {end:#x})"))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            out,
            "const UR{i}: UnicodeRanges = UnicodeRanges::new(&[{entries}]);"
        )
        .unwrap();
    }
    out
}

fn catalog(families: &[FamilyMetadata], ranges: &[String]) -> String {
    let mut out = unicode_range_consts(ranges);
    out.push('\n');
    let mut all = String::new();
    let mut seen = HashSet::new();

    for f in families {
        let ident = family_ident(&f.id);
        assert!(
            seen.insert(ident.clone()),
            "family constant collision on `{ident}`"
        );

        let subsets = f
            .subsets
            .iter()
            .map(|s| format!("Subset::{}", subset_variant(s)))
            .collect::<Vec<_>>()
            .join(", ");
        let weights = f
            .weights
            .iter()
            .map(u16::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        let styles = f
            .styles
            .iter()
            .map(|s| format!("Style::{}", style_variant(s)))
            .collect::<Vec<_>>()
            .join(", ");
        let unicode_ranges = f
            .unicode_range
            .iter()
            .map(|(subset, idx)| format!("(Subset::{}, UR{idx})", subset_variant(subset)))
            .collect::<Vec<_>>()
            .join(", ");

        writeln!(
            out,
            "/// `{}`: `{}`, `{}`.",
            f.family, f.category, f.license
        )
        .unwrap();
        writeln!(out, "pub const {ident}: Family = Family {{").unwrap();
        writeln!(out, "    id: {:?},", f.id).unwrap();
        writeln!(out, "    ident: {ident:?},").unwrap();
        writeln!(out, "    name: {:?},", f.family).unwrap();
        writeln!(out, "    subsets: &[{subsets}],").unwrap();
        writeln!(out, "    weights: &[{weights}],").unwrap();
        writeln!(out, "    styles: &[{styles}],").unwrap();
        writeln!(
            out,
            "    default_subset: Subset::{},",
            subset_variant(&f.def_subset)
        )
        .unwrap();
        writeln!(out, "    variable: {},", f.variable).unwrap();
        writeln!(out, "    category: {:?},", f.category).unwrap();
        writeln!(out, "    license: {:?},", f.license).unwrap();
        writeln!(out, "    provider: {:?},", f.provider).unwrap();
        writeln!(out, "    unicode_ranges: &[{unicode_ranges}],").unwrap();
        out.push_str("};\n\n");

        writeln!(all, "    &{ident},").unwrap();
    }

    out.push_str("/// Every family in the vendored catalog, sorted by [`id`](Family::id).\n");
    write!(out, "pub static ALL: &[&Family] = &[\n{all}];\n").unwrap();
    out
}
