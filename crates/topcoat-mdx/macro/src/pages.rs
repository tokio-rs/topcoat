use std::path::Path;

use heck::ToKebabCase;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::{Ident, LitStr, Path as SynPath, spanned::Spanned};
use topcoat_core_grammar::paths::{
    topcoat_context, topcoat_error, topcoat_inventory, topcoat_mdx, topcoat_router, topcoat_view,
};
use topcoat_mdx_grammar::{
    parse::get_parse_options,
    walker::{FrontmatterFormat, extract_frontmatter},
};

use crate::compile::parse_and_walk_mdx;

// ---------------------------------------------------------------------------
// mdx_pages! helpers
// ---------------------------------------------------------------------------

/// Derives a route path for a discovered `.mdx` or `.md` file.
///
/// Given the scan directory, the resolved file path, and an optional prefix,
/// computes the route path: applies the prefix, then appends the relative
/// directory structure and kebab-cased filename stem.
///
/// A file named `index` is the exception. It stands for the directory holding
/// it, so `posts/my-post/index.mdx` is `/my-post` rather than
/// `/my-post/index`. This lets a post keep its images and partials in one
/// directory without the route repeating itself.
pub(crate) fn derive_route_path(scan_dir: &Path, file_path: &Path, prefix: Option<&str>) -> String {
    let relative = file_path
        .strip_prefix(scan_dir)
        .unwrap_or(file_path)
        .to_string_lossy();

    // Remove .mdx or .md extension.
    let mut route = relative.into_owned();
    if let Some(ext) = std::path::Path::new(&route)
        .extension()
        .and_then(|e| e.to_str())
    {
        if ext.eq_ignore_ascii_case("mdx") {
            route.truncate(route.len() - 4);
        } else if ext.eq_ignore_ascii_case("md") {
            route.truncate(route.len() - 2);
        }
    }

    // Kebab-case the filename stem (last path component).
    let parts: Vec<&str> = route.rsplitn(2, '/').collect();
    let (dir_part, stem) = if parts.len() == 2 {
        (Some(parts[1]), parts[0])
    } else {
        (None, parts[0])
    };
    let kebab_stem = stem.to_kebab_case();
    let is_index = kebab_stem == "index";

    let mut path_parts: Vec<String> = Vec::new();
    if let Some(dir) = dir_part {
        let kebab_dir: Vec<String> = dir.split('/').map(str::to_kebab_case).collect();
        path_parts.push(kebab_dir.join("/"));
    }
    if !is_index {
        path_parts.push(kebab_stem);
    }

    let relative_route = path_parts.join("/");

    match prefix {
        // An index file at the scan root leaves nothing to append, so the
        // prefix is the whole route.
        Some(p) if relative_route.is_empty() => p.trim_end_matches('/').to_owned(),
        Some(p) => format!("{}/{}", p.trim_end_matches('/'), relative_route),
        None => format!("/{relative_route}"),
    }
}

/// Rejects two scanned files that derive the same route.
///
/// Route derivation is not injective: `one.mdx` and `one/index.mdx` both give
/// `/one`, and kebab-casing maps `my_post.mdx` and `my-post.mdx` onto
/// `/my-post`. Without this check the file that wins is whichever the
/// directory walk reached last, which is neither stable nor visible.
///
/// # Errors
///
/// Returns an error naming the route and both files that claim it.
pub(crate) fn check_route_collisions(
    scanned: &[(String, std::path::PathBuf)],
    span: Span,
) -> Result<(), syn::Error> {
    let mut claimed: std::collections::HashMap<&str, &std::path::Path> =
        std::collections::HashMap::with_capacity(scanned.len());

    for (route, file_path) in scanned {
        if let Some(previous) = claimed.insert(route, file_path) {
            return Err(syn::Error::new(
                span,
                format!(
                    "mdx_pages! derives the route '{route}' from two files: '{}' and '{}'; \
                     rename one of them",
                    previous.display(),
                    file_path.display()
                ),
            ));
        }
    }

    Ok(())
}

/// A scanned page entry: file path, parsed content, and frontmatter data.
pub(crate) struct MdxPageEntry {
    pub(crate) file_path: std::path::PathBuf,
}

/// Scans a directory for `.mdx` and `.md` files, returning valid entries.
pub(crate) fn scan_directory(
    canonical_scan_dir: &Path,
    canonical_manifest: &Path,
    _span: Span,
) -> Vec<MdxPageEntry> {
    let mut entries = Vec::new();

    for entry in ignore::Walk::new(canonical_scan_dir) {
        let Ok(entry) = entry else {
            continue;
        };

        let file_path = entry.path().to_path_buf();

        // Only process .mdx and .md files.
        let is_target = file_path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("mdx") || ext.eq_ignore_ascii_case("md"));
        if !is_target {
            continue;
        }

        // Security: verify resolved path stays within manifest directory.
        let resolved_path = file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone());
        if !resolved_path.starts_with(canonical_manifest) {
            continue;
        }

        entries.push(MdxPageEntry { file_path });
    }

    entries
}

/// The tokens `mdx_pages!` emits for its content index.
pub(crate) struct BuiltIndex {
    /// One `MdxIndexEntry` literal per scanned page.
    pub(crate) items: Vec<proc_macro2::TokenStream>,
    /// Statics holding parsed frontmatter, emitted only when the macro was
    /// given a `frontmatter = Type` argument.
    pub(crate) meta_statics: Vec<proc_macro2::TokenStream>,
    /// For each scanned page, the function reading its parsed frontmatter, so
    /// that a wrapper component can be handed the same value the index holds.
    /// Positions line up with the entries passed in.
    pub(crate) meta_readers: Vec<Option<Ident>>,
}

/// Builds index entries from scanned pages.
///
/// For each page, extracts title/date/tags/excerpt from frontmatter
/// using generic `serde_value::Value` parsing, and derives the full
/// route path from the scan directory, file location, and optional prefix.
///
/// When `frontmatter` names a type, each page that carries frontmatter also
/// gets a static holding it deserialized into that type, parsed on first use.
///
/// # Errors
///
/// Returns an error when a scanned file carries frontmatter that fails to
/// deserialize, naming the file so the author can find it.
pub(crate) fn build_index(
    scan_dir: &Path,
    entries: &[MdxPageEntry],
    prefix: Option<&str>,
    frontmatter_type: Option<&SynPath>,
    span: Span,
) -> Result<BuiltIndex, syn::Error> {
    let mut index_items = Vec::new();
    let mut meta_statics = Vec::new();
    let mut meta_readers = vec![None; entries.len()];

    for (position, entry) in entries.iter().enumerate() {
        let resolved = entry
            .file_path
            .canonicalize()
            .unwrap_or_else(|_| entry.file_path.clone());

        let Ok(content) = std::fs::read_to_string(&resolved) else {
            continue;
        };

        // Parse the markdown content.
        let options = get_parse_options();
        let Ok(root) = markdown::to_mdast(&content, &options) else {
            continue;
        };

        // Extract frontmatter data.
        let frontmatter = extract_frontmatter(&root);

        // The whole block, tagged with its syntax, so a consumer can read
        // fields the named ones do not cover.
        let (fm_raw_lit, fm_format_expr) = match &frontmatter {
            Some((fm_content, FrontmatterFormat::Yaml)) => (
                LitStr::new(fm_content, span),
                quote! { #topcoat_mdx::MdxFrontmatterFormat::Yaml },
            ),
            Some((fm_content, FrontmatterFormat::Toml)) => (
                LitStr::new(fm_content, span),
                quote! { #topcoat_mdx::MdxFrontmatterFormat::Toml },
            ),
            None => (
                LitStr::new("", span),
                quote! { #topcoat_mdx::MdxFrontmatterFormat::None },
            ),
        };

        // Count words in the body only. The frontmatter node spans the whole
        // block including its delimiters, so its end offset starts the body.
        let body_start = match &root {
            markdown::mdast::Node::Root(r) if frontmatter.is_some() => r
                .children
                .first()
                .and_then(|node| node.position().map(|p| p.end.offset))
                .unwrap_or(0),
            _ => 0,
        };
        let word_count = content[body_start..].split_whitespace().count();

        // With `frontmatter = Type`, deserializing happens once per page on
        // first use. Names are positional: two files can share a stem, and the
        // panic message names the file rather than the static.
        let mut meta_fn_expr = quote! { None };
        if let (Some(meta_type), Some((fm_content, format))) = (frontmatter_type, &frontmatter) {
            let static_name = Ident::new(&format!("__MDX_META_{position}"), span);
            let reader_name = Ident::new(&format!("__mdx_meta_{position}"), span);
            let raw_lit = LitStr::new(fm_content, span);
            let file_lit = LitStr::new(&resolved.display().to_string(), span);
            let parse = match format {
                FrontmatterFormat::Yaml => quote! { #topcoat_mdx::__private::parse_yaml },
                FrontmatterFormat::Toml => quote! { #topcoat_mdx::__private::parse_toml },
            };

            meta_statics.push(quote! {
                static #static_name: ::std::sync::LazyLock<#meta_type> =
                    ::std::sync::LazyLock::new(|| #parse(#raw_lit, #file_lit));

                fn #reader_name() -> &'static #meta_type {
                    &#static_name
                }
            });
            meta_fn_expr = quote! { Some(#reader_name) };
            meta_readers[position] = Some(reader_name);
        }

        // Derive slug from file stem using kebab-case. An index file is named
        // after the directory it stands for, matching its route: every index
        // file would otherwise answer to the slug "index".
        let stem = resolved
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("page");
        let slug = if stem.eq_ignore_ascii_case("index") {
            resolved
                .parent()
                .and_then(std::path::Path::file_name)
                .and_then(|name| name.to_str())
                .unwrap_or(stem)
                .to_kebab_case()
        } else {
            stem.to_kebab_case()
        };
        // Intentional leak: slugs/paths are small and the index array requires
        // &'static str. Leaking avoids complex lifetime management.
        let slug_str: &'static str = Box::leak(slug.into_boxed_str());

        // Derive full route path (prefix + relative subdirs + kebab stem).
        let route_path_str = derive_route_path(scan_dir, &entry.file_path, prefix);
        let path_str: &'static str = Box::leak(route_path_str.into_boxed_str());

        // Extract known fields from frontmatter using generic deserialization.
        let (title_expr, date_expr, excerpt_expr, tags_expr) = if let Some((fm_content, format)) =
            &frontmatter
        {
            let display = resolved.display();
            let deserialized: serde_value::Value = if matches!(format, FrontmatterFormat::Yaml) {
                serde_saphyr::from_str(fm_content).map_err(|e| {
                    syn::Error::new(
                        span,
                        format!(
                            "mdx_pages! cannot deserialize YAML frontmatter in '{display}': {e}"
                        ),
                    )
                })?
            } else {
                toml::from_str(fm_content).map_err(|e| {
                    syn::Error::new(
                        span,
                        format!(
                            "mdx_pages! cannot deserialize TOML frontmatter in '{display}': {e}"
                        ),
                    )
                })?
            };

            let title = extract_string_field(&deserialized, "title");
            let date = extract_string_field(&deserialized, "date");
            let excerpt = extract_string_field(&deserialized, "excerpt");
            let tags = extract_tags_field(&deserialized);

            (
                title
                    .map(|s| quote! { Some(#s) })
                    .unwrap_or(quote! { None }),
                date.map(|s| quote! { Some(#s) }).unwrap_or(quote! { None }),
                excerpt
                    .map(|s| quote! { Some(#s) })
                    .unwrap_or(quote! { None }),
                tags,
            )
        } else {
            (
                quote! { None },
                quote! { None },
                quote! { None },
                quote! { &[] },
            )
        };

        index_items.push(quote! {
            #topcoat_mdx::MdxIndexEntry {
                slug: #slug_str,
                path: #path_str,
                title: #title_expr,
                date: #date_expr,
                excerpt: #excerpt_expr,
                tags: #tags_expr,
                frontmatter_raw: #fm_raw_lit,
                frontmatter_format: #fm_format_expr,
                word_count: #word_count,
                meta_fn: #meta_fn_expr,
            }
        });
    }

    Ok(BuiltIndex {
        items: index_items,
        meta_statics,
        meta_readers,
    })
}

/// Extract a string field from a `serde_value::Value` Map.
fn extract_string_field(value: &serde_value::Value, field: &str) -> Option<LitStr> {
    if let serde_value::Value::Map(entries) = value
        && let Some(serde_value::Value::String(s)) =
            entries.get(&serde_value::Value::String(field.to_string()))
    {
        Some(LitStr::new(s, Span::call_site()))
    } else {
        None
    }
}

/// Extract a tags field (sequence of strings) from a `serde_value::Value` Map.
fn extract_tags_field(value: &serde_value::Value) -> proc_macro2::TokenStream {
    if let serde_value::Value::Map(entries) = value
        && let Some(serde_value::Value::Seq(items)) =
            entries.get(&serde_value::Value::String("tags".to_string()))
    {
        let tag_lits: Vec<LitStr> = items
            .iter()
            .filter_map(|v| {
                if let serde_value::Value::String(s) = v {
                    Some(LitStr::new(s, Span::call_site()))
                } else {
                    None
                }
            })
            .collect();

        if !tag_lits.is_empty() {
            return quote! { &[#(#tag_lits),*] };
        }
    }
    quote! { &[] }
}

/// Generates page registration tokens for a single `.mdx` or `.md` file.
///
/// Mirrors the logic in `mdx_page!` but supports the components, overrides,
/// and wrapper arguments from `mdx_pages!`.
pub(crate) fn generate_page_registration(
    file_path: &Path,
    route_path: &str,
    components: &[(String, SynPath)],
    overrides: &[(&'static str, SynPath)],
    wrapper: Option<&SynPath>,
    meta_arg: Option<&proc_macro2::TokenStream>,
    span: Span,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let path_display = file_path.to_string_lossy();
    let resolved = file_path.canonicalize().map_err(|e| {
        syn::Error::new(
            span,
            format!("mdx_pages! cannot resolve path '{path_display}': {e}"),
        )
    })?;

    let content = std::fs::read_to_string(&resolved).map_err(|e| {
        syn::Error::new(
            span,
            format!("mdx_pages! cannot read '{path_display}': {e}"),
        )
    })?;

    let result = parse_and_walk_mdx(components, overrides, wrapper, &content, "mdx_pages!", span)?;

    // Generate unique identifiers from file stem.
    // Use snake_case for identifiers (valid Rust) but the route path
    // (passed as argument) may use kebab-case.
    let file_stem = resolved
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("page")
        .to_kebab_case()
        .replace('-', "_");
    let render_fn_name = Ident::new(&format!("__mdx_pages_render_{file_stem}"), span);
    let unit_name = Ident::new(&format!("__mdx_pages_{file_stem}"), span);
    let route_path_lit = LitStr::new(route_path, span);

    let view_tokens = &result.view_tokens;

    // Apply wrapper if requested.
    let render_body = if result.has_wrapper {
        let wrapper_path = result.wrapper_path.as_ref().unwrap();
        // A wrapper takes a `meta` prop exactly when the macro was given a
        // frontmatter type, whether or not a given page carries frontmatter,
        // so one wrapper serves the whole directory. Spanning the call at the
        // `wrapper = ...` argument points the resulting error at the component
        // the author named, rather than at tokens they cannot see.
        let meta_prop = meta_arg.map(|arg| quote_spanned! { wrapper_path.span() => .meta(#arg) });
        quote! {
            {
                use #topcoat_view::Component;
                let props = #wrapper_path::props_builder()
                    .child(#view_tokens)
                    #meta_prop
                    .build();
                Component::render(#wrapper_path::default(), __cx, props).await
            }
        }
    } else {
        quote! { Ok(#view_tokens?) }
    };

    // Only submit to the link-time inventory when discovery is enabled, so
    // that `mdx_pages!` also compiles without the `discover` feature.
    let submit =
        cfg!(feature = "discover").then(|| quote! { #topcoat_inventory::submit!(ERASED); });

    Ok(quote! {
        #[allow(clippy::needless_question_mark)]
        const _: () = {
            fn #render_fn_name(
                __cx: &#topcoat_context::Cx,
                body: #topcoat_router::Body,
            ) -> ::std::pin::Pin<
                Box<dyn ::core::future::Future<Output = #topcoat_error::Result<#topcoat_view::View>> + Send + '_>
            > {
                ::std::boxed::Box::pin(async move {
                    #render_body
                })
            }

            #[allow(non_camel_case_types)]
            struct #unit_name;

            const ERASED: #topcoat_router::PageFn = #topcoat_router::PageFn::const_new(
                #topcoat_router::OwnedMethods::One(#topcoat_router::Method::GET),
                ::std::borrow::Cow::Borrowed(#topcoat_router::Path::new(#route_path_lit)),
                #render_fn_name,
            );

            impl ::core::convert::From<#unit_name> for #topcoat_router::PageFn {
                fn from(_: #unit_name) -> Self {
                    ERASED
                }
            }

            #submit
        };
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{check_route_collisions, derive_route_path};

    fn route(relative: &str, prefix: Option<&str>) -> String {
        derive_route_path(
            Path::new("/content"),
            &Path::new("/content").join(relative),
            prefix,
        )
    }

    #[test]
    fn flat_file_keeps_its_stem() {
        assert_eq!(route("hello-world.mdx", None), "/hello-world");
        assert_eq!(route("hello-world.mdx", Some("/blog")), "/blog/hello-world");
    }

    #[test]
    fn stem_is_kebab_cased() {
        assert_eq!(route("MyPost.mdx", Some("/blog")), "/blog/my-post");
    }

    #[test]
    fn nested_file_keeps_its_directory() {
        assert_eq!(
            route("archive/older.mdx", Some("/blog")),
            "/blog/archive/older"
        );
    }

    // A file named `index` stands for its directory, so the directory itself
    // is the route rather than gaining a redundant final segment.
    #[test]
    fn index_file_takes_its_directory_route() {
        assert_eq!(route("my-post/index.mdx", Some("/blog")), "/blog/my-post");
        assert_eq!(route("my-post/index.md", Some("/blog")), "/blog/my-post");
    }

    #[test]
    fn index_file_collapses_through_several_directories() {
        assert_eq!(
            route("archive/old-post/index.mdx", Some("/blog")),
            "/blog/archive/old-post"
        );
    }

    // A sibling of an index file is unaffected.
    #[test]
    fn sibling_of_index_keeps_its_own_segment() {
        assert_eq!(
            route("my-post/appendix.mdx", Some("/blog")),
            "/blog/my-post/appendix"
        );
    }

    #[test]
    fn index_at_the_scan_root_is_the_prefix() {
        assert_eq!(route("index.mdx", Some("/blog")), "/blog");
        assert_eq!(route("index.mdx", None), "/");
    }

    #[test]
    fn distinct_routes_do_not_collide() {
        let scanned = [
            ("/blog/one".to_owned(), PathBuf::from("/content/one.mdx")),
            ("/blog/two".to_owned(), PathBuf::from("/content/two.mdx")),
        ];
        assert!(check_route_collisions(&scanned, proc_macro2::Span::call_site()).is_ok());
    }

    // `one.mdx` and `one/index.mdx` both claim `/blog/one`. Left unchecked the
    // winner would depend on the order the directory happened to be walked.
    #[test]
    fn index_file_colliding_with_a_flat_sibling_is_an_error() {
        let scanned = [
            ("/blog/one".to_owned(), PathBuf::from("/content/one.mdx")),
            (
                "/blog/one".to_owned(),
                PathBuf::from("/content/one/index.mdx"),
            ),
        ];
        let error = check_route_collisions(&scanned, proc_macro2::Span::call_site())
            .expect_err("the two files claim one route");
        let message = error.to_string();
        assert!(message.contains("/blog/one"), "names the route: {message}");
        assert!(message.contains("one.mdx"), "names both files: {message}");
        assert!(
            message.contains("one/index.mdx"),
            "names both files: {message}"
        );
    }

    // Kebab-casing can map two distinct filenames onto one route, which was
    // silent before this check.
    #[test]
    fn kebab_cased_names_colliding_is_an_error() {
        let scanned = [
            (
                "/blog/my-post".to_owned(),
                PathBuf::from("/content/my-post.mdx"),
            ),
            (
                "/blog/my-post".to_owned(),
                PathBuf::from("/content/my_post.mdx"),
            ),
        ];
        assert!(check_route_collisions(&scanned, proc_macro2::Span::call_site()).is_err());
    }
}
