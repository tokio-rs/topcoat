//! Proc-macro crate for `topcoat-mdx`.
//!
//! Provides the `compile_mdx!` macro that reads `.mdx` or `.md` files at compile time,
//! parses them with `markdown-rs`, walks the mdast into `view!` AST nodes,
//! and emits tokens. Also provides `mdx_page!` for registering `.mdx` or `.md` files
//! as page routes.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod compile;
mod input;
mod pages;

use std::path::Path;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::{Ident, LitStr, Path as SynPath, spanned::Spanned};
use topcoat_core_grammar::paths::{
    topcoat_context, topcoat_error, topcoat_inventory, topcoat_mdx, topcoat_router, topcoat_view,
};
use topcoat_mdx_grammar::walker::FrontmatterFormat;

use crate::{
    compile::compile_mdx_file,
    input::{CompileMdxInput, MdxPageInput, MdxPagesInput},
    pages::{
        build_index, check_route_collisions, derive_route_path, generate_page_registration,
        scan_directory,
    },
};

/// Rejects `frontmatter = Type` when the runtime support it expands into is
/// not compiled in.
///
/// Deserializing frontmatter into a caller's type happens at runtime, unlike
/// the rest of MDX compilation, so it lives behind a feature the caller opts
/// into. Without this check the generated code fails on a missing module,
/// which says nothing about the feature.
fn check_frontmatter_feature(frontmatter: Option<&SynPath>) -> Result<(), syn::Error> {
    match frontmatter {
        Some(path) if !cfg!(feature = "frontmatter") => Err(syn::Error::new(
            path.span(),
            "`frontmatter = Type` needs the `mdx-frontmatter` feature of `topcoat` \
             (or the `frontmatter` feature of `topcoat-mdx`); reading `frontmatter_raw` \
             from the index works without it",
        )),
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// compile_mdx! proc-macro
// ---------------------------------------------------------------------------

#[doc = include_str!("../docs/compile_mdx.md")]
#[proc_macro]
#[allow(clippy::missing_panics_doc)]
pub fn compile_mdx(tokens: TokenStream) -> TokenStream {
    let input = match syn::parse::<CompileMdxInput>(tokens) {
        Ok(i) => i,
        Err(e) => {
            let msg = format!(
                "compile_mdx! expects a string literal path, optionally preceded by a component registry: {e}"
            );
            return syn::Error::new(Span::call_site(), msg)
                .to_compile_error()
                .into();
        }
    };

    let (components, wrapper, overrides, lit_str) = match input {
        CompileMdxInput::TwoArgs {
            components,
            wrapper,
            lit_str,
        } => (
            components,
            wrapper,
            Vec::<(&'static str, SynPath)>::new(),
            lit_str,
        ),
        CompileMdxInput::TwoArgsWithOverrides {
            components,
            overrides,
            wrapper,
            lit_str,
        } => (components, wrapper, overrides, lit_str),
        CompileMdxInput::OneArg { lit_str } => (Vec::new(), None, Vec::new(), lit_str),
    };

    let path_str = lit_str.value();

    let result = match compile_mdx_file(
        &components,
        &overrides,
        wrapper.as_ref(),
        &path_str,
        lit_str.span(),
    ) {
        Ok(r) => r,
        Err(e) => return e.to_compile_error().into(),
    };

    let view_tokens = &result.view_tokens;

    // Build the final output tokens. When no wrapper is requested, emit exactly
    // what view_tokens contains (the original async { Ok(...) }.await pattern).
    // When a wrapper is requested, wrap the inner view tokens in a Component
    // render call using __cx from the enclosing scope.
    let final_tokens = if result.has_wrapper {
        let wrapper_path = result.wrapper_path.as_ref().unwrap();
        quote! {
            async {
                {
                    use #topcoat_view::Component;
                    let props = #wrapper_path::props_builder().child(#view_tokens).build();
                    Component::render(#wrapper_path::default(), __cx, props).await
                }
            }.await
        }
    } else {
        quote! { #view_tokens }
    };

    // Frontmatter is parsed to keep it out of the rendered body, but
    // `compile_mdx!` is an expression macro and has nowhere to hand it back.
    // Read frontmatter through `mdx_pages!` and `MdxIndexEntry` instead.
    quote! { #final_tokens }.into()
}

// ---------------------------------------------------------------------------
// mdx_page! proc-macro
// ---------------------------------------------------------------------------

#[doc = include_str!("../docs/mdx_page.md")]
#[proc_macro]
#[allow(clippy::too_many_lines, clippy::missing_panics_doc)]
pub fn mdx_page(tokens: TokenStream) -> TokenStream {
    let input = match syn::parse::<MdxPageInput>(tokens) {
        Ok(i) => i,
        Err(e) => {
            return syn::Error::new(
                Span::call_site(),
                format!("mdx_page! expects: route_path, file_path [, components = {{ ... }}]: {e}"),
            )
            .to_compile_error()
            .into();
        }
    };

    if let Err(e) = check_frontmatter_feature(input.frontmatter.as_ref()) {
        return e.to_compile_error().into();
    }

    let route_path = &input.route_path;
    let file_path = &input.file_path;
    let path_str = file_path.value();

    let components: Vec<(String, SynPath)> = input.components.unwrap_or_default();
    let overrides: Vec<(&'static str, SynPath)> = input.overrides.unwrap_or_default();
    let result = match compile_mdx_file(
        &components,
        &overrides,
        input.wrapper.as_ref(),
        &path_str,
        file_path.span(),
    ) {
        Ok(r) => r,
        Err(e) => return e.to_compile_error().into(),
    };

    let view_tokens = &result.view_tokens;

    // With `frontmatter = Type`, the page's frontmatter is deserialized once
    // on first use and handed to the wrapper.
    // The prop is passed whenever a frontmatter type was named, so a wrapper
    // written for one page works for a page without frontmatter too.
    let (meta_static, meta_value) = match (&input.frontmatter, &result.frontmatter) {
        (Some(meta_type), Some((fm_content, format))) => {
            let raw_lit = LitStr::new(fm_content, file_path.span());
            let parse = match format {
                FrontmatterFormat::Yaml => quote! { #topcoat_mdx::__private::parse_yaml },
                FrontmatterFormat::Toml => quote! { #topcoat_mdx::__private::parse_toml },
            };
            let statik = quote! {
                static __MDX_PAGE_META: ::std::sync::LazyLock<#meta_type> =
                    ::std::sync::LazyLock::new(|| #parse(#raw_lit, #path_str));
            };
            (Some(statik), Some(quote! { Some(&*__MDX_PAGE_META) }))
        }
        (Some(_), None) => (None, Some(quote! { None })),
        (None, _) => (None, None),
    };
    // Spanned at the wrapper argument, so a component without a `meta` prop
    // reports the error where the author named it.
    let meta_prop = match (&input.wrapper, &meta_value) {
        (Some(wrapper_path), Some(value)) => {
            Some(quote_spanned! { wrapper_path.span() => .meta(#value) })
        }
        _ => None,
    };

    // Apply wrapper if requested, emitting a Component::render() call using `cx`.
    let render_body = if result.has_wrapper {
        let wrapper_path = result.wrapper_path.as_ref().unwrap();
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

    // Generate unique identifiers from file stem.
    let file_stem = Path::new(&path_str)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("page")
        .replace('-', "_");
    let render_fn_name = Ident::new(&format!("__mdx_render_{file_stem}"), file_path.span());
    let unit_name = Ident::new(&format!("__mdx_page_{file_stem}"), file_path.span());

    // Only submit to the link-time inventory when discovery is enabled, so
    // that `mdx_page!` also compiles without the `discover` feature.
    let submit =
        cfg!(feature = "discover").then(|| quote! { #topcoat_inventory::submit!(ERASED); });

    // Emit the page registration.
    quote! {
        #[allow(clippy::needless_question_mark)]
        const _: () = {
            #meta_static

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
                ::std::borrow::Cow::Borrowed(#topcoat_router::Path::new(#route_path)),
                #render_fn_name,
            );

            impl ::core::convert::From<#unit_name> for #topcoat_router::PageFn {
                fn from(_: #unit_name) -> Self {
                    ERASED
                }
            }

            #submit
        };
    }
    .into()
}

// ---------------------------------------------------------------------------
// mdx_pages! proc-macro
// ---------------------------------------------------------------------------

#[doc = include_str!("../docs/mdx_pages.md")]
#[proc_macro]
pub fn mdx_pages(tokens: TokenStream) -> TokenStream {
    let input = match syn::parse::<MdxPagesInput>(tokens) {
        Ok(i) => i,
        Err(e) => {
            return syn::Error::new(
                Span::call_site(),
                format!("mdx_pages! expects: directory_path [, prefix = \"/path\"]: {e}"),
            )
            .to_compile_error()
            .into();
        }
    };

    if let Err(e) = check_frontmatter_feature(input.frontmatter.as_ref()) {
        return e.to_compile_error().into();
    }

    let dir_str = input.directory_path.value();
    let span = input.directory_path.span();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_owned());
    let scan_dir = Path::new(&manifest_dir).join(&dir_str);

    // Validate scan directory exists.
    if !scan_dir.is_dir() {
        return syn::Error::new(
            span,
            format!(
                "mdx_pages! directory '{dir_str}' does not exist (resolved: {})",
                scan_dir.display()
            ),
        )
        .to_compile_error()
        .into();
    }

    let canonical_scan_dir = scan_dir.canonicalize().map_err(|e| {
        syn::Error::new(
            span,
            format!("mdx_pages! cannot canonicalize directory '{dir_str}': {e}"),
        )
    });

    let canonical_scan_dir = match canonical_scan_dir {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };

    let canonical_manifest = std::path::Path::new(&manifest_dir)
        .canonicalize()
        .map_err(|e| {
            syn::Error::new(
                span,
                format!("mdx_pages! cannot canonicalize CARGO_MANIFEST_DIR '{manifest_dir}': {e}"),
            )
        });

    let canonical_manifest = match canonical_manifest {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };

    // Security: verify scan directory stays within manifest directory before
    // enumeration. Per-file guards at line ~1106 catch escaping
    // entries, but rejecting the whole directory avoids unnecessary traversal,
    // prevents external file paths from leaking through diagnostics, and
    // matches compile_mdx_file which validates before reading.
    if !canonical_scan_dir.starts_with(&canonical_manifest) {
        return syn::Error::new(
            span,
            format!("mdx_pages! scan directory '{dir_str}' resolves outside CARGO_MANIFEST_DIR"),
        )
        .to_compile_error()
        .into();
    }

    let prefix = input.prefix.as_ref().map(syn::LitStr::value);
    let components: Vec<(String, SynPath)> = input.components.unwrap_or_default();
    let overrides: Vec<(&'static str, SynPath)> = input.overrides.unwrap_or_default();

    // Scan directory for .mdx and .md files.
    let page_entries = scan_directory(&canonical_scan_dir, &canonical_manifest, span);

    // Build index entries from scanned pages. This runs first because it also
    // names the statics holding parsed frontmatter, which the wrapper of each
    // registered route is handed.
    let index = match build_index(
        &canonical_scan_dir,
        &page_entries,
        prefix.as_deref(),
        input.frontmatter.as_ref(),
        span,
    ) {
        Ok(index) => index,
        Err(e) => return e.to_compile_error().into(),
    };
    let index_entries = index.items;
    let meta_statics = index.meta_statics;

    // A wrapper's prop shape follows the macro argument, not the individual
    // page: with `frontmatter = Type` every page passes a `meta` prop, and a
    // page without frontmatter passes `None`. One wrapper then serves a
    // directory whose pages do not all carry frontmatter.
    let meta_args: Vec<Option<proc_macro2::TokenStream>> = index
        .meta_readers
        .iter()
        .map(|reader| {
            input.frontmatter.as_ref().map(|_| {
                reader
                    .as_ref()
                    .map_or_else(|| quote! { None }, |reader| quote! { Some(#reader()) })
            })
        })
        .collect();

    // Two files can derive one route, through an index file next to a
    // same-named sibling or through kebab-casing. Reject that before
    // registering anything, so the outcome does not depend on walk order.
    let scanned: Vec<(String, std::path::PathBuf)> = page_entries
        .iter()
        .map(|entry| {
            (
                derive_route_path(&canonical_scan_dir, &entry.file_path, prefix.as_deref()),
                entry.file_path.clone(),
            )
        })
        .collect();
    if let Err(e) = check_route_collisions(&scanned, span) {
        return e.to_compile_error().into();
    }

    // Generate route registrations.
    let route_results: Vec<proc_macro2::TokenStream> = page_entries
        .iter()
        .zip(&scanned)
        .zip(&meta_args)
        .map(|((entry, (route_path, _)), meta_arg)| {
            match generate_page_registration(
                &entry.file_path,
                route_path,
                &components,
                &overrides,
                input.wrapper.as_ref(),
                meta_arg.as_ref(),
                span,
            ) {
                Ok(ts) => ts,
                Err(e) => e.to_compile_error(),
            }
        })
        .collect();

    // Without `frontmatter = Type` the entries carry no parsed metadata, and
    // the index is `MdxIndexEntry<()>` through the type parameter's default.
    let entry_type = input.frontmatter.as_ref().map_or_else(
        || quote! { #topcoat_mdx::MdxIndexEntry },
        |meta_type| quote! { #topcoat_mdx::MdxIndexEntry<#meta_type> },
    );

    // Derive a stable identifier from the directory path for the index name.
    let index_suffix = dir_str
        .replace([std::path::MAIN_SEPARATOR, '/', '-'], "_")
        .to_uppercase();

    let index_const_name = Ident::new(&format!("MDX_INDEX_{index_suffix}"), span);
    let index_fn_name = Ident::new(&format!("mdx_index_{}", index_suffix.to_lowercase()), span);

    // Combine route results into a single TokenStream.
    let route_tokens = route_results
        .into_iter()
        .collect::<proc_macro2::TokenStream>();

    // Build index const using the collected entries.
    let index_const_tokens = quote! {
        &[
            #(#index_entries),*
        ]
    };

    quote! {
        #route_tokens

        #(#meta_statics)*

        #[allow(clippy::approx_constant)]
        const #index_const_name: &'static [#entry_type] = #index_const_tokens;

        #[allow(clippy::approx_constant)]
        fn #index_fn_name() -> &'static [#entry_type] {
            #index_const_name
        }
    }
    .into()
}
