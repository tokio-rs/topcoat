//! Generates the font catalog from the vendored Fontsource `/v1/fonts`
//! response (`fonts.json`).
//!
//! Two files are written into `OUT_DIR`:
//!
//! - `subset.rs` — the [`Subset`] enum, one variant per distinct subset across
//!   the whole catalog, with `as_str`/`from_id` conversions.
//! - `families.rs` — one `Family` constant per family plus the `ALL`
//!   slice.

use std::{
    collections::{BTreeSet, HashSet},
    env,
    fmt::Write as _,
    fs,
    path::Path,
};

use heck::{ToPascalCase, ToShoutySnakeCase};
use serde::Deserialize;

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
}

fn main() {
    println!("cargo::rerun-if-changed=fonts.json");

    let json = fs::read_to_string("fonts.json").expect("read fonts.json");
    let mut families: Vec<FamilyMetadata> = serde_json::from_str(&json).expect("parse fonts.json");
    families.sort_by(|a, b| a.id.cmp(&b.id));

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    fs::write(Path::new(&out_dir).join("subset.rs"), subsets(&families)).expect("write subset.rs");
    fs::write(Path::new(&out_dir).join("families.rs"), catalog(&families))
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
    out.push_str("            _ => return None,\n        })\n    }\n}\n");
    out
}

fn catalog(families: &[FamilyMetadata]) -> String {
    let mut out = String::new();
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

        writeln!(
            out,
            "/// `{}` — `{}`, `{}`.",
            f.family, f.category, f.license
        )
        .unwrap();
        writeln!(out, "pub const {ident}: Family = Family {{").unwrap();
        writeln!(out, "    id: {:?},", f.id).unwrap();
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
        out.push_str("};\n\n");

        writeln!(all, "    &{ident},").unwrap();
    }

    out.push_str("/// Every family in the vendored catalog, sorted by [`id`](Family::id).\n");
    write!(out, "pub static ALL: &[&Family] = &[\n{all}];\n").unwrap();
    out
}
