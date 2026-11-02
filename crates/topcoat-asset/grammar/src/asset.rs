mod option;

pub use option::*;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    LitStr, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};
use topcoat_core_grammar::{
    common::{CallSite, UnparsedExpr},
    paths::topcoat_asset,
};

/// Arguments to `asset!`: a path or URL, optionally followed by options.
///
/// ```text
/// asset!("./logo.png")
/// asset!("https://example.com/font.woff2", rename: "primary")
/// ```
///
/// Parsing accepts options that are still being typed, such as a name without a
/// value, so editors can complete them.
pub struct Asset {
    pub path: UnparsedExpr,
    pub comma_token: Option<Token![,]>,
    pub options: Punctuated<AssetOption, Token![,]>,
}

impl Asset {
    /// Generates the asset handle, resolving the path from `call_site`.
    ///
    /// The expansion is a valid `Asset` expression even when the input has errors,
    /// which are reported next to it. Without a source file, as in rust-analyzer,
    /// the files on disk are not checked and an unresolved path is left empty.
    #[must_use]
    pub fn expand(&self, call_site: &CallSite) -> TokenStream {
        let (path, location, error) = match self.locate(call_site) {
            Ok((path, location)) => (path, location, None),
            Err(error) => (String::new(), String::new(), Some(error.to_compile_error())),
        };
        let fields = self.options.iter().map(AssetOption::field);
        quote! {{
            #error
            #[allow(clippy::needless_update)]
            const OPTIONS: #topcoat_asset::AssetOptions = #topcoat_asset::AssetOptions {
                #(#fields,)*
                ..#topcoat_asset::AssetOptions::NONE
            };
            const ID: #topcoat_asset::AssetId = #topcoat_asset::AssetId::new(
                ::core::env!("CARGO_CRATE_NAME"),
                #location,
                &OPTIONS,
            );
            const LEN: usize = #topcoat_asset::RawAsset::encoded_len(#path, &OPTIONS);
            static ENCODED: [u8; LEN] = #topcoat_asset::RawAsset::encode(ID, #path, &OPTIONS);
            #topcoat_asset::Asset::new(&ENCODED)
        }}
    }

    /// Returns the path or URL to embed and the location that identifies the asset.
    ///
    /// The location is a path relative to the crate's manifest directory when the
    /// file is inside it, so the ID does not depend on where the crate is built.
    fn locate(&self, call_site: &CallSite) -> syn::Result<(String, String)> {
        if self.path.is_empty() {
            return Err(syn::Error::new(Span::call_site(), "expected a path or URL"));
        }
        let lit: LitStr = syn::parse2(self.path.tokens.clone()).map_err(|_| {
            syn::Error::new_spanned(&self.path, "expected a string literal path or URL")
        })?;
        let value = lit.value();
        if value.starts_with("http://") || value.starts_with("https://") {
            return Ok((value.clone(), value));
        }

        let has_source_file = call_site.source_file().is_some();
        let path = match call_site.resolve(&value) {
            Ok(path) => path,
            Err(message) if has_source_file => return Err(syn::Error::new(lit.span(), message)),
            // rust-analyzer does not expand the macro again when files change, so
            // an error here would stay after it is fixed.
            Err(_) => return Ok((String::new(), String::new())),
        };
        if has_source_file && !path.is_file() {
            return Err(syn::Error::new(
                lit.span(),
                format!("cannot find the file `{}`", path.display()),
            ));
        }
        let location = path.strip_prefix(call_site.manifest_dir()).unwrap_or(&path);
        Ok((
            path.to_string_lossy().into_owned(),
            location.to_string_lossy().replace('\\', "/"),
        ))
    }
}

/// Generates the asset handle using the current [call site](CallSite::current).
impl ToTokens for Asset {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match CallSite::current() {
            Ok(call_site) => self.expand(&call_site),
            Err(err) => syn::Error::new(Span::call_site(), err).to_compile_error(),
        });
    }
}

impl Parse for Asset {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path = input.parse()?;
        let comma_token: Option<Token![,]> = input.parse()?;
        let options = match comma_token {
            Some(_) => Punctuated::parse_terminated(input)?,
            None => Punctuated::new(),
        };
        Ok(Self {
            path,
            comma_token,
            options,
        })
    }
}

/// Prints the call on one line if it fits. Otherwise, puts the path and each option
/// on their own lines, followed by a comma.
#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Asset {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use topcoat_core_grammar::pretty::TextMode;

        self.path.pretty_print(printer);

        let Some(comma_token) = &self.comma_token else {
            return;
        };
        if self.options.is_empty() {
            printer.move_cursor(comma_token.span.end());
            return;
        }

        comma_token.pretty_print(printer);
        for (index, pair) in self.options.pairs().enumerate() {
            printer.scan_same_line_trivia();
            printer.scan_break();
            " ".pretty_print(printer);
            printer.scan_trivia(true, true);
            pair.value().pretty_print(printer);
            if pair.punct().is_some() {
                printer.scan_no_break_trivia();
            }
            match pair.punct() {
                Some(punct) if index < self.options.len() - 1 => punct.pretty_print(printer),
                _ => {
                    printer.scan_text(",".into(), TextMode::Break);
                    printer.advance_cursor(",");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use tempfile::TempDir;

    use super::*;

    fn parse(input: &str) -> syn::Result<Asset> {
        syn::parse_str(input)
    }

    /// Creates a test crate containing the given relative file paths.
    fn crate_with(files: &[&str]) -> TempDir {
        let dir = TempDir::new().unwrap();
        for file in files {
            let path = dir.path().join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "").unwrap();
        }
        dir
    }

    fn call_site(dir: &TempDir, source_file: Option<&str>) -> CallSite {
        let source_file = source_file.map(|file| dir.path().join(file));
        CallSite::new(dir.path(), source_file.as_deref(), None).unwrap()
    }

    /// Expands `input` from `src/lib.rs` in `dir`.
    fn expand(dir: &TempDir, input: &str) -> String {
        parse(input)
            .unwrap()
            .expand(&call_site(dir, Some("src/lib.rs")))
            .to_string()
    }

    fn tokens(input: &str) -> String {
        input.parse::<TokenStream>().unwrap().to_string()
    }

    #[test]
    fn parses_options_being_typed() {
        for input in [
            r#""a.png", ren"#,
            r#""a.png", rename:"#,
            r#""a.png", rename:, extension: "x""#,
        ] {
            assert!(parse(input).is_ok(), "{input}");
        }
    }

    #[test]
    fn writes_each_option_as_a_field() {
        let dir = crate_with(&["a.png"]);
        let expanded = expand(&dir, r#""a.png", rename: "logo", extension: "svg""#);
        assert!(expanded.contains("rename :"), "{expanded}");
        assert!(expanded.contains("extension :"), "{expanded}");
    }

    #[test]
    fn passes_option_values_on_as_written() {
        let dir = crate_with(&["a.png"]);
        let expanded = expand(
            &dir,
            r#""a.png", rename: NAME, extension: concat!("w", "off2")"#,
        );
        assert!(expanded.contains(&tokens("Borrowed (NAME)")), "{expanded}");
        assert!(
            expanded.contains(&tokens(r#"Borrowed (concat!("w", "off2"))"#)),
            "{expanded}"
        );
        assert!(!expanded.contains("compile_error"), "{expanded}");
    }

    #[test]
    fn a_missing_value_after_a_colon_is_an_error() {
        let dir = crate_with(&["a.png"]);
        let expanded = expand(&dir, r#""a.png", rename:"#);
        assert!(expanded.contains("rename :"), "{expanded}");
        assert!(expanded.contains("compile_error"), "{expanded}");
    }

    #[test]
    fn a_name_without_a_colon_is_a_shorthand_field() {
        // Only the name keeps its span, so editors complete it as a field name.
        let dir = crate_with(&["a.png"]);
        let expanded = expand(&dir, r#""a.png", ren"#);
        assert!(expanded.contains("ren ,"), "{expanded}");
        assert!(!expanded.contains("compile_error"), "{expanded}");
    }

    #[test]
    fn checks_the_format_of_a_literal_checksum() {
        let dir = crate_with(&["a.png"]);
        for checksum in ["md5:0", "sha256:abc", &format!("sha256:{}", "A".repeat(64))] {
            let input = format!(r#""a.png", checksum: {checksum:?}"#);
            assert!(expand(&dir, &input).contains("compile_error"), "{input}");
        }
        let input = format!(r#""a.png", checksum: "sha256:{}""#, "0".repeat(64));
        assert!(!expand(&dir, &input).contains("compile_error"));
    }

    #[test]
    fn embeds_the_resolved_path_and_a_relative_location() {
        let dir = crate_with(&["assets/a.png", "src/b.png"]);
        let root = std::path::absolute(dir.path()).unwrap();
        for (input, path, location) in [
            (
                r#""assets/a.png""#,
                root.join("assets/a.png"),
                "assets/a.png",
            ),
            (r#""./b.png""#, root.join("src/b.png"), "src/b.png"),
        ] {
            let asset = parse(input).unwrap();
            let call_site = call_site(&dir, Some("src/lib.rs"));
            assert_eq!(
                asset.locate(&call_site).unwrap(),
                (path.to_string_lossy().into_owned(), location.to_owned()),
            );
        }
    }

    #[test]
    fn leaves_urls_unresolved() {
        let dir = crate_with(&[]);
        let url = "https://example.com/font.woff2";
        let asset = parse(&format!("{url:?}")).unwrap();
        let call_site = call_site(&dir, Some("src/lib.rs"));
        assert_eq!(
            asset.locate(&call_site).unwrap(),
            (url.to_owned(), url.to_owned())
        );
    }

    #[test]
    fn a_missing_file_is_an_error() {
        let dir = crate_with(&[]);
        assert!(expand(&dir, r#""missing.png""#).contains("compile_error"));
    }

    #[test]
    fn checks_no_files_without_a_source_file() {
        let dir = crate_with(&[]);
        for input in [r#""missing.png""#, r#""./missing.png""#] {
            let expanded = parse(input)
                .unwrap()
                .expand(&call_site(&dir, None))
                .to_string();
            assert!(!expanded.contains("compile_error"), "{expanded}");
        }
    }

    #[test]
    fn a_path_that_is_not_a_literal_still_expands_to_an_asset() {
        let dir = crate_with(&[]);
        for input in ["", "PATH", r#"concat!("a", "b")"#] {
            let expanded = expand(&dir, input);
            assert!(expanded.contains("compile_error"), "{input}");
            assert!(expanded.contains(&tokens("Asset :: new")), "{input}");
        }
    }

    #[test]
    fn the_location_does_not_depend_on_the_build_directory() {
        let first = crate_with(&["assets/a.png"]);
        let second = crate_with(&["assets/a.png"]);
        let location = |dir: &TempDir| {
            parse(r#""assets/a.png""#)
                .unwrap()
                .locate(&call_site(dir, Some("src/lib.rs")))
                .unwrap()
                .1
        };
        assert_ne!(first.path(), second.path());
        assert_eq!(location(&first), location(&second));
        assert!(Path::new(&location(&first)).is_relative());
    }
}
