//! Regenerates `fonts.json` from the Fontsource API.
//!
//! Run with `cargo run -p topcoat-font --features fontsource-refresh --bin refresh`.
//!
//! The catalog list (`/v1/fonts`) already carries every field the build script
//! needs except the per-subset unicode ranges, which only appear in each
//! family's detail (`/v1/fonts/{id}`). This fetches the list, augments each
//! entry with its `unicodeRange`, and writes a trimmed, deduplicated catalog.
//!
//! The same range string repeats across thousands of families, so the ranges
//! are interned: distinct range specs are collected into a top-level `ranges`
//! table and each family references them by index. The numbered CJK subset
//! blocks (`[0]`, `[1]`, ...) are dropped: their ranges are per-font and opaque,
//! and keeping them would bloat the catalog many times over.

use std::{
    collections::{BTreeMap, HashMap},
    io::Write as _,
    path::Path,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
};

use serde::{Deserialize, Serialize};

/// Base URL of the Fontsource API.
const API: &str = "https://api.fontsource.org/v1";
/// Number of detail requests to run concurrently.
const WORKERS: usize = 16;
/// Attempts per detail request before giving up.
const ATTEMPTS: usize = 3;

/// A catalog entry as fetched: every field the build script reads, plus the
/// per-subset unicode-range strings pulled from the family detail.
#[derive(Deserialize)]
struct Font {
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
    kind: String,
    #[serde(default)]
    unicode_range: BTreeMap<String, String>,
}

/// Just the `unicodeRange` map, parsed from a family's detail endpoint.
#[derive(Deserialize)]
struct Detail {
    #[serde(rename = "unicodeRange", default)]
    unicode_range: BTreeMap<String, String>,
}

/// The vendored catalog: an interned `ranges` table and the families that
/// reference it.
#[derive(Serialize)]
struct Catalog {
    ranges: Vec<String>,
    fonts: Vec<FontOut>,
}

/// A catalog entry as written, with `unicodeRange` mapping each subset to an
/// index into [`Catalog::ranges`].
#[derive(Serialize)]
struct FontOut {
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
    kind: String,
    #[serde(rename = "unicodeRange")]
    unicode_range: BTreeMap<String, usize>,
}

fn main() {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .user_agent(concat!("topcoat-font/", env!("CARGO_PKG_VERSION")))
        .build()
        .into();

    eprintln!("fetching catalog list...");
    let mut fonts: Vec<Font> = get_json(&agent, &format!("{API}/fonts"));
    fonts.sort_by(|a, b| a.id.cmp(&b.id));
    eprintln!("{} families; fetching unicode ranges...", fonts.len());

    let done = AtomicUsize::new(0);
    let total = fonts.len();
    thread::scope(|scope| {
        let chunk = fonts.len().div_ceil(WORKERS);
        for slice in fonts.chunks_mut(chunk) {
            let agent = agent.clone();
            let done = &done;
            scope.spawn(move || {
                for font in slice {
                    let detail: Detail = get_json(&agent, &format!("{API}/fonts/{}", font.id));
                    font.unicode_range = detail
                        .unicode_range
                        .into_iter()
                        .filter(|(subset, _)| is_named_subset(subset))
                        .collect();
                    let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                    if n.is_multiple_of(100) || n == total {
                        eprintln!("  {n}/{total}");
                    }
                }
            });
        }
    });

    let catalog = intern(fonts);
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts.json");
    let json = serde_json::to_string(&catalog).expect("serialize catalog");
    let mut file = std::fs::File::create(&path).expect("create fonts.json");
    file.write_all(json.as_bytes()).expect("write fonts.json");
    eprintln!(
        "wrote {} ({} families, {} distinct ranges)",
        path.display(),
        catalog.fonts.len(),
        catalog.ranges.len()
    );
}

/// Whether a subset is a named subset (`latin`, `cyrillic`, ...) rather than a
/// numbered CJK block (`[0]`, `[12]`, ...).
fn is_named_subset(subset: &str) -> bool {
    !(subset.starts_with('[')
        && subset.ends_with(']')
        && subset[1..subset.len() - 1]
            .bytes()
            .all(|b| b.is_ascii_digit()))
}

/// Collects the distinct range specs into a sorted table and rewrites each
/// family's `unicodeRange` to reference them by index.
fn intern(fonts: Vec<Font>) -> Catalog {
    let mut ranges: Vec<String> = fonts
        .iter()
        .flat_map(|font| font.unicode_range.values().cloned())
        .collect();
    ranges.sort();
    ranges.dedup();
    let index: HashMap<&str, usize> = ranges
        .iter()
        .enumerate()
        .map(|(i, spec)| (spec.as_str(), i))
        .collect();

    let fonts = fonts
        .into_iter()
        .map(|font| FontOut {
            unicode_range: font
                .unicode_range
                .iter()
                .map(|(subset, spec)| (subset.clone(), index[spec.as_str()]))
                .collect(),
            id: font.id,
            family: font.family,
            subsets: font.subsets,
            weights: font.weights,
            styles: font.styles,
            def_subset: font.def_subset,
            variable: font.variable,
            category: font.category,
            license: font.license,
            kind: font.kind,
        })
        .collect();

    Catalog { ranges, fonts }
}

/// Fetches and deserializes JSON from `url`, retrying transient failures.
fn get_json<T: for<'de> Deserialize<'de>>(agent: &ureq::Agent, url: &str) -> T {
    let mut last_err = None;
    for attempt in 0..ATTEMPTS {
        match agent.get(url).call() {
            Ok(mut response) => match response.body_mut().read_to_string() {
                Ok(body) => match serde_json::from_str(&body) {
                    Ok(value) => return value,
                    Err(err) => last_err = Some(err.to_string()),
                },
                Err(err) => last_err = Some(err.to_string()),
            },
            Err(err) => last_err = Some(err.to_string()),
        }
        thread::sleep(std::time::Duration::from_millis(250 * (attempt as u64 + 1)));
    }
    panic!("failed to fetch {url}: {}", last_err.unwrap_or_default());
}
