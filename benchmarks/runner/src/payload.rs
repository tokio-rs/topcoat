//! Client payload: what a browser downloads to show a page.
//!
//! Every response is fetched uncompressed and compressed here with the same settings, so apps
//! are compared on content rather than on server compression settings.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    io::Write,
    path::Path,
    time::Duration,
};

use anyhow::{Context, Result, bail};

use crate::{
    html::Page,
    results::{Asset, PagePayload, Sizes},
};

/// The pages whose payload is measured, as `(name, path)`.
pub fn pages(detail_id: u32) -> Vec<(&'static str, String)> {
    vec![
        ("home", "/".to_owned()),
        ("detail", format!("/products/{detail_id}")),
    ]
}

/// Upper bound on fetched assets per page, in case a bundle references itself in a loop.
const MAX_ASSETS: usize = 300;

/// Measures one page of the app at `base_url`.
pub fn measure(
    base_url: &str,
    page: &str,
    path: &str,
    extra_assets: &[String],
    cart_count: u32,
) -> Result<PagePayload> {
    let agent = agent();
    let cookie = format!("cart_count={cart_count}");
    let (html, _) = fetch(&agent, &format!("{base_url}{path}"), &cookie)?;
    let html_text = String::from_utf8_lossy(&html).into_owned();
    let document = Page::parse(&html_text);

    let mut assets = vec![Asset {
        path: path.to_owned(),
        category: "html".to_owned(),
        source: "document".to_owned(),
        sizes: sizes(&html),
    }];
    let mut missing = Vec::new();
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::new();
    for reference in document.references() {
        if let Some(url) = resolve(path, &reference.url) {
            queue.push_back((url, reference.category, "html"));
        }
    }
    for url in extra_assets {
        if let Some(url) = resolve(path, url) {
            queue.push_back((url, None, "manifest"));
        }
    }
    let inline_scripts = document.inline_scripts();
    for script in &inline_scripts {
        for url in script_references(script) {
            if let Some(url) = resolve(path, &url) {
                queue.push_back((url, None, "import"));
            }
        }
    }

    while let Some((url, hint, source)) = queue.pop_front() {
        if !seen.insert(url.clone()) || seen.len() > MAX_ASSETS {
            continue;
        }
        let Ok((body, content_type)) = fetch(&agent, &format!("{base_url}{url}"), &cookie) else {
            // String literals in scripts are only guesses; one that does not load is not an
            // asset. Everything the HTML or the manifest references must load.
            if source != "import" {
                missing.push(url);
            }
            continue;
        };
        let category = categorize(&url, hint, content_type.as_deref());
        if category == "js" {
            let text = String::from_utf8_lossy(&body);
            for found in script_references(&text) {
                if let Some(found) = resolve(&url, &found) {
                    queue.push_back((found, None, "import"));
                }
            }
        }
        assets.push(Asset {
            path: url,
            category: category.to_owned(),
            source: source.to_owned(),
            sizes: sizes(&body),
        });
    }

    let mut totals: BTreeMap<String, Sizes> = ["html", "js", "wasm", "css", "other"]
        .iter()
        .map(|name| ((*name).to_owned(), Sizes::default()))
        .collect();
    let mut total = Sizes::default();
    for asset in &assets {
        *totals.entry(asset.category.clone()).or_default() += asset.sizes;
        total += asset.sizes;
    }
    Ok(PagePayload {
        page: page.to_owned(),
        path: path.to_owned(),
        assets,
        totals,
        total,
        inline_script_bytes: inline_scripts.iter().map(|s| s.len() as u64).sum(),
        inline_style_bytes: document.inline_style_bytes() as u64,
        missing,
    })
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .new_agent()
}

/// Fetches `url` and returns the decoded body and the content type.
fn fetch(agent: &ureq::Agent, url: &str, cookie: &str) -> Result<(Vec<u8>, Option<String>)> {
    let mut response = agent
        .get(url)
        .header("Cookie", cookie)
        .call()
        .with_context(|| format!("GET {url} failed"))?;
    if response.status() != 200 {
        bail!("GET {url} returned {}", response.status());
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body = response
        .body_mut()
        .with_config()
        .limit(256 * 1024 * 1024)
        .read_to_vec()?;
    Ok((body, content_type))
}

/// Uncompressed, gzip (level 9) and brotli (quality 11, 4 MiB window) sizes.
pub fn sizes(bytes: &[u8]) -> Sizes {
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    gzip.write_all(bytes).expect("writing to a Vec cannot fail");
    let gzip = gzip.finish().expect("writing to a Vec cannot fail").len();

    let mut brotli = Vec::new();
    {
        let mut writer = brotli::CompressorWriter::new(&mut brotli, 4096, 11, 22);
        writer
            .write_all(bytes)
            .expect("writing to a Vec cannot fail");
    }
    Sizes {
        raw: bytes.len() as u64,
        gzip: gzip as u64,
        brotli: brotli.len() as u64,
    }
}

fn categorize(url: &str, hint: Option<&'static str>, content_type: Option<&str>) -> &'static str {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let extension = path
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase());
    match extension.as_deref() {
        Some("js" | "mjs" | "cjs") => return "js",
        Some("wasm") => return "wasm",
        Some("css") => return "css",
        _ => {}
    }
    let content_type = content_type.unwrap_or("").to_ascii_lowercase();
    if content_type.contains("javascript") {
        "js"
    } else if content_type.contains("wasm") {
        "wasm"
    } else if content_type.contains("css") {
        "css"
    } else {
        hint.unwrap_or("other")
    }
}

/// Resolves `reference` against the path of the document or script that contains it.
/// Returns `None` for other origins and non-HTTP URLs.
fn resolve(base: &str, reference: &str) -> Option<String> {
    let reference = reference.trim();
    if reference.is_empty() || reference.starts_with("data:") || reference.starts_with("//") {
        return None;
    }
    if let Some(rest) = reference
        .strip_prefix("http://")
        .or_else(|| reference.strip_prefix("https://"))
    {
        let host = rest.split('/').next().unwrap_or("");
        if !(host.starts_with("127.0.0.1") || host.starts_with("localhost")) {
            return None;
        }
        return Some(format!(
            "/{}",
            rest.split_once('/').map_or("", |(_, path)| path)
        ));
    }
    if reference.contains(':') && !reference.starts_with('/') && !reference.starts_with('.') {
        return None;
    }
    let reference = reference.split('#').next().unwrap_or(reference);
    let joined = if reference.starts_with('/') {
        reference.to_owned()
    } else {
        let base = base.split(['?', '#']).next().unwrap_or(base);
        let dir = base.rsplit_once('/').map_or("", |(dir, _)| dir);
        format!("{dir}/{reference}")
    };
    let (path, query) = joined
        .split_once('?')
        .map_or((joined.as_str(), None), |(path, query)| (path, Some(query)));
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    let mut out = format!("/{}", segments.join("/"));
    if let Some(query) = query {
        out.push('?');
        out.push_str(query);
    }
    Some(out)
}

/// String literals in a script that name another script or a WASM module: relative or
/// absolute paths ending in `.js` or `.mjs`, and any name ending in `.wasm`.
fn script_references(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let quote = bytes[index];
        if !matches!(quote, b'"' | b'\'' | b'`') {
            index += 1;
            continue;
        }
        let start = index + 1;
        let mut end = start;
        while end < bytes.len() && bytes[end] != quote && bytes[end] != b'\n' && end - start < 300 {
            end += 1;
        }
        if end < bytes.len() && bytes[end] == quote {
            let literal = &source[start..end];
            let is_path =
                literal.starts_with('/') || literal.starts_with("./") || literal.starts_with("../");
            let extension = Path::new(literal).extension().and_then(|ext| ext.to_str());
            let is_script = matches!(extension, Some("js" | "mjs"));
            let is_wasm = extension == Some("wasm");
            let plain = !literal.contains(|c: char| c.is_whitespace() || c == '$' || c == '{');
            if plain && ((is_path && is_script) || is_wasm) {
                found.push(literal.to_owned());
            }
            index = end + 1;
        } else {
            index = start;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_and_absolute_references() {
        assert_eq!(
            resolve("/products/1", "app.js").as_deref(),
            Some("/products/app.js")
        );
        assert_eq!(
            resolve("/pkg/app.js", "./app_bg.wasm").as_deref(),
            Some("/pkg/app_bg.wasm")
        );
        assert_eq!(
            resolve("/pkg/a/b.js", "../c.js").as_deref(),
            Some("/pkg/c.js")
        );
        assert_eq!(resolve("/", "/x.css?v=1").as_deref(), Some("/x.css?v=1"));
        assert_eq!(
            resolve("/", "http://127.0.0.1:3000/y.js").as_deref(),
            Some("/y.js")
        );
        assert_eq!(resolve("/", "https://cdn.example.com/y.js"), None);
        assert_eq!(resolve("/", "data:text/css,a"), None);
    }

    #[test]
    fn finds_wasm_and_static_imports_in_scripts() {
        let source = r#"import init from '/pkg/app.js'; init({ module_or_path: "/pkg/app.wasm" });
            new URL('app_bg.wasm', import.meta.url); const s = "not a path.js"; import("./chunk.mjs")"#;
        let found = script_references(source);
        for expected in ["/pkg/app.js", "/pkg/app.wasm", "app_bg.wasm", "./chunk.mjs"] {
            assert!(
                found.iter().any(|f| f == expected),
                "{expected} missing from {found:?}"
            );
        }
        assert!(!found.iter().any(|f| f.contains("not a path")));
    }

    #[test]
    fn compressed_sizes_are_smaller_for_repetitive_content() {
        let content = "<div class=\"card\">hello</div>".repeat(200);
        let sizes = sizes(content.as_bytes());
        assert_eq!(sizes.raw, content.len() as u64);
        assert!(sizes.gzip < sizes.raw / 10);
        assert!(sizes.brotli < sizes.raw / 10);
    }
}
