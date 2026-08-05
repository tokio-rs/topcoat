use syn::{
    Ident, LitStr, Path as SynPath, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

// ---------------------------------------------------------------------------
// compile_mdx! input parsing
// ---------------------------------------------------------------------------

/// A single `Ident => Path` pair in the component registry braced block.
pub(crate) struct CompPair {
    name: Ident,
    path: SynPath,
}

impl Parse for CompPair {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let _: Token![=>] = input.parse()?;
        let path: SynPath = input.parse()?;
        Ok(Self { name, path })
    }
}

/// Input for `compile_mdx!`: either two-arg (registry + path) or one-arg (path).
pub(crate) enum CompileMdxInput {
    TwoArgs {
        components: Vec<(String, SynPath)>,
        wrapper: Option<SynPath>,
        lit_str: LitStr,
    },
    TwoArgsWithOverrides {
        components: Vec<(String, SynPath)>,
        overrides: Vec<(&'static str, SynPath)>,
        wrapper: Option<SynPath>,
        lit_str: LitStr,
    },
    OneArg {
        lit_str: LitStr,
    },
}

/// A single `"tag" => Path` pair in the overrides braced block.
pub(crate) struct OverridePair {
    tag: LitStr,
    path: SynPath,
}

impl Parse for OverridePair {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let tag: LitStr = input.parse()?;
        let _: Token![=>] = input.parse()?;
        let path: SynPath = input.parse()?;
        Ok(Self { tag, path })
    }
}

/// Parses a braced block of `CompPair`s from a `ParseStream`.
pub(crate) fn parse_component_braces(content: ParseStream) -> syn::Result<Vec<(String, SynPath)>> {
    let pairs = Punctuated::<CompPair, Token![,]>::parse_terminated(content)?;
    Ok(pairs
        .into_iter()
        .map(|p| (p.name.to_string(), p.path))
        .collect())
}

impl Parse for CompileMdxInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Pattern 1: { Ident => Path, ... } [, overrides = { "tag" => Path, ... }]
        // [, wrapper = Path], "path.mdx": direct braced block
        if input.peek(syn::token::Brace) {
            let content;
            syn::braced!(content in input);
            let components = parse_component_braces(&content)?;
            let overrides = parse_optional_overrides(input)?;
            let wrapper = parse_optional_wrapper(input)?;
            input.parse::<Token![,]>()?;
            let lit_str: LitStr = input.parse()?;
            return if overrides.is_empty() {
                Ok(Self::TwoArgs {
                    components,
                    wrapper,
                    lit_str,
                })
            } else {
                Ok(Self::TwoArgsWithOverrides {
                    components,
                    overrides,
                    wrapper,
                    lit_str,
                })
            };
        }

        // Pattern 2: mdx_components!{ Ident => Path, ... } [, overrides = { "tag" => Path, ... }]
        // [, wrapper = Path], "path.mdx": mdx_components! macro_rules! invocation.
        if input.peek(Ident) {
            let fork = input.fork();
            let maybe_ident: Ident = fork.parse()?;
            if fork.peek(Token![!])
                && fork.peek2(syn::token::Brace)
                && maybe_ident == "mdx_components"
            {
                let _macro_name: Ident = input.parse()?;
                let _bang: Token![!] = input.parse()?;
                let content;
                syn::braced!(content in input);
                let components = parse_component_braces(&content)?;
                let overrides = parse_optional_overrides(input)?;
                let wrapper = parse_optional_wrapper(input)?;
                input.parse::<Token![,]>()?;
                let lit_str: LitStr = input.parse()?;
                return if overrides.is_empty() {
                    Ok(Self::TwoArgs {
                        components,
                        wrapper,
                        lit_str,
                    })
                } else {
                    Ok(Self::TwoArgsWithOverrides {
                        components,
                        overrides,
                        wrapper,
                        lit_str,
                    })
                };
            }
        }

        // Pattern 3: "path.mdx", the backward compatible one-arg form
        let lit_str: LitStr = input.parse()?;
        Ok(Self::OneArg { lit_str })
    }
}

/// Parses an optional `wrapper = Path` from a `ParseStream`.
/// Returns `None` if no wrapper keyword is found.
pub(crate) fn parse_optional_wrapper(input: ParseStream) -> syn::Result<Option<SynPath>> {
    let fork = input.fork();
    if !fork.peek(Token![,]) {
        return Ok(None);
    }
    let _: Token![,] = fork.parse()?;
    if !fork.peek(Ident) {
        return Ok(None);
    }
    let maybe_kw: Ident = fork.parse()?;
    if maybe_kw != "wrapper" || !fork.peek(Token![=]) {
        return Ok(None);
    }
    // Consume from the actual stream.
    input.parse::<Token![,]>()?;
    let _kw: Ident = input.parse()?;
    input.parse::<Token![=]>()?;
    let path: SynPath = input.parse()?;
    Ok(Some(path))
}

/// Parses an optional `overrides = { "tag" => Path, ... }` from a `ParseStream`.
/// Returns an empty vector if no overrides keyword is found.
pub(crate) fn parse_optional_overrides(
    input: ParseStream,
) -> syn::Result<Vec<(&'static str, SynPath)>> {
    let fork = input.fork();
    if !fork.peek(Token![,]) {
        return Ok(Vec::new());
    }
    let _: Token![,] = fork.parse()?;
    if !fork.peek(Ident) {
        return Ok(Vec::new());
    }
    let maybe_kw: Ident = fork.parse()?;
    if maybe_kw != "overrides" || !fork.peek(Token![=]) {
        return Ok(Vec::new());
    }
    // Consume from the actual stream.
    input.parse::<Token![,]>()?;
    let _kw: Ident = input.parse()?;
    input.parse::<Token![=]>()?;
    let content;
    syn::braced!(content in input);
    let pairs = Punctuated::<OverridePair, Token![,]>::parse_terminated(&content)?;
    Ok(pairs
        .into_iter()
        .map(|p| {
            // Intentional leak: override tag names (e.g. "a", "img", "pre")
            // are small and the WalkContext requires &'static str. Leaking
            // avoids complex lifetime gymnastics at proc-macro expand time.
            // This is acceptable because override tags are declared per-
            // invocation and their total heap cost is negligible.
            (String::leak(p.tag.value()) as &'static str, p.path)
        })
        .collect())
}

// ---------------------------------------------------------------------------
// mdx_page! input parsing
// ---------------------------------------------------------------------------

/// Input for `mdx_page!`: (`route_path`, `file_path`, [overrides = {...}],
/// [components = {...}], [wrapper = Path], [frontmatter = Type])
pub(crate) struct MdxPageInput {
    pub(crate) route_path: LitStr,
    pub(crate) file_path: LitStr,
    pub(crate) overrides: Option<Vec<(&'static str, SynPath)>>,
    pub(crate) components: Option<Vec<(String, SynPath)>>,
    pub(crate) wrapper: Option<SynPath>,
    pub(crate) frontmatter: Option<SynPath>,
}

impl Parse for MdxPageInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let route_path: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let file_path: LitStr = input.parse()?;

        let mut overrides = None;
        let mut components = None;
        let mut wrapper = None;
        let mut frontmatter = None;

        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            // A trailing comma after the last argument ends the list.
            if input.is_empty() {
                break;
            }
            let kw: Ident = input.parse()?;
            if kw == "frontmatter" {
                input.parse::<Token![=]>()?;
                frontmatter = Some(input.parse()?);
            } else if kw == "overrides" {
                input.parse::<Token![=]>()?;
                let content;
                syn::braced!(content in input);
                let pairs = Punctuated::<OverridePair, Token![,]>::parse_terminated(&content)?;
                overrides = Some(
                    pairs
                        .into_iter()
                        .map(|p| {
                            // Intentional leak: override tag names are small and
                            // WalkContext requires &'static str.
                            (String::leak(p.tag.value()) as &'static str, p.path)
                        })
                        .collect(),
                );
            } else if kw == "components" {
                input.parse::<Token![=]>()?;
                let content;
                syn::braced!(content in input);
                components = Some(parse_component_braces(&content)?);
            } else if kw == "wrapper" {
                input.parse::<Token![=]>()?;
                wrapper = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    kw.span(),
                    "expected `overrides = { ... }`, `components = { ... }`, `wrapper = Path`, or `frontmatter = Type`, found something else",
                ));
            }
        }

        Ok(Self {
            route_path,
            file_path,
            overrides,
            components,
            wrapper,
            frontmatter,
        })
    }
}

// ---------------------------------------------------------------------------
// mdx_pages! input parsing
// ---------------------------------------------------------------------------

/// Input for `mdx_pages!`: (`directory_path`, prefix = "/optional/prefix",
/// components = {...}, overrides = {...}, wrapper = Path, frontmatter = Type)
pub(crate) struct MdxPagesInput {
    pub(crate) directory_path: LitStr,
    pub(crate) prefix: Option<LitStr>,
    pub(crate) components: Option<Vec<(String, SynPath)>>,
    pub(crate) overrides: Option<Vec<(&'static str, SynPath)>>,
    pub(crate) wrapper: Option<SynPath>,
    pub(crate) frontmatter: Option<SynPath>,
}

impl Parse for MdxPagesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let directory_path: LitStr = input.parse()?;

        let mut prefix = None;
        let mut components = None;
        let mut overrides = None;
        let mut wrapper = None;
        let mut frontmatter = None;

        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            // A trailing comma after the last argument ends the list.
            if input.is_empty() {
                break;
            }
            let kw: Ident = input.parse()?;
            if kw == "frontmatter" {
                input.parse::<Token![=]>()?;
                frontmatter = Some(input.parse()?);
            } else if kw == "prefix" {
                input.parse::<Token![=]>()?;
                prefix = Some(input.parse()?);
            } else if kw == "components" {
                input.parse::<Token![=]>()?;
                let content;
                syn::braced!(content in input);
                components = Some(parse_component_braces(&content)?);
            } else if kw == "overrides" {
                input.parse::<Token![=]>()?;
                let content;
                syn::braced!(content in input);
                let pairs = Punctuated::<OverridePair, Token![,]>::parse_terminated(&content)?;
                overrides = Some(
                    pairs
                        .into_iter()
                        .map(|p| {
                            // Intentional leak: override tag names are small and
                            // WalkContext requires &'static str.
                            (String::leak(p.tag.value()) as &'static str, p.path)
                        })
                        .collect(),
                );
            } else if kw == "wrapper" {
                input.parse::<Token![=]>()?;
                wrapper = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    kw.span(),
                    "expected `prefix = \"/path\"`, `components = { ... }`, `overrides = { ... }`, `wrapper = Path`, or `frontmatter = Type`",
                ));
            }
        }

        Ok(Self {
            directory_path,
            prefix,
            components,
            overrides,
            wrapper,
            frontmatter,
        })
    }
}
