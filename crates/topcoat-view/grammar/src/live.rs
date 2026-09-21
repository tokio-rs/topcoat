use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::Brace,
};
use topcoat_core_grammar::{
    ParseOption,
    paths::{topcoat_context, topcoat_core, topcoat_view},
};

use crate::{
    leading_cx::LeadingCx,
    view::{
        View,
        hir::{LowerView, ViewBuilder},
    },
};

pub struct Live {
    pub cx: Option<LeadingCx>,
    pub body: LiveBody,
}

impl Parse for Live {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            cx: if LeadingCx::peek(input) && !LiveBranch::peek(input) {
                Some(input.parse()?)
            } else {
                None
            },
            body: input.parse()?,
        })
    }
}

impl ToTokens for Live {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // Read the input span: line!() and column!() would name the outer
        // view! invocation and merge its distinct live! sites.
        let start = self.body.span().unwrap_or_else(Span::call_site).start();
        let line = u32::try_from(start.line).expect("source line fits in u32");
        let column = u32::try_from(start.column + 1).expect("source column fits in u32");
        let site = quote! {
            const {
                #topcoat_core::identity::SiteKey::new(::core::file!(), #line, #column, 0)
            }
        };
        let borrow_cx = self.cx.as_ref().map(|_| quote! { let __cx = &__cx; });
        let cx = if self.cx.is_some() {
            quote! { &__cx }
        } else {
            quote! { __cx }
        };
        let identity = quote! { #topcoat_context::identity(#cx) };
        let view = match &self.body {
            LiveBody::Single(body) => quote! {
                #topcoat_view::internal::LiveView::new(
                    __region,
                    #topcoat_view::pass(#cx),
                    async move {
                        #borrow_cx
                        #(#body)*
                    },
                )
            },
            LiveBody::Branches(branches) => {
                LiveBranch::expand(branches, &cx, borrow_cx.as_ref())
            }
        };
        let view = quote! {{
            let __region = #topcoat_view::RegionId::new(#identity, #site);
            #view
        }};
        match &self.cx {
            Some(cx) => {
                let cx = &cx.cx;
                quote! {{
                    let __cx: #topcoat_context::Cx = (#cx).clone();
                    #view
                }}
                .to_tokens(tokens);
            }
            None => view.to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Live {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use topcoat_core_grammar::pretty::PrettyPrint;

        if let Some(cx) = &self.cx {
            cx.pretty_print(printer);
        }
        let items: Vec<&dyn PrettyPrint> = match &self.body {
            LiveBody::Single(body) => body.iter().map(|stmt| stmt as &dyn PrettyPrint).collect(),
            LiveBody::Branches(branches) => branches
                .iter()
                .map(|branch| branch as &dyn PrettyPrint)
                .collect(),
        };
        for (index, item) in items.iter().enumerate() {
            item.pretty_print(printer);
            if index < items.len() - 1 {
                printer.scan_same_line_trivia();
                printer.scan_break();
                " ".pretty_print(printer);
                printer.scan_trivia(true, true);
            }
        }
        // Branches are block arms and lay out one per line like a `match`.
        if items.len() > 1 || matches!(self.body, LiveBody::Branches(_)) {
            printer.scan_force_break();
        }
    }
}

/// The body of a `live!` region: one body serving both phases, or a branch
/// per phase.
pub enum LiveBody {
    /// A single body, run in the initial phase until its first emission and
    /// again in full in the connected phase.
    Single(Vec<syn::Stmt>),
    /// A body per phase, each named after a `Pass` variant. The `Initial`
    /// branch is required; the `Connected` branch is optional.
    Branches(Vec<LiveBranch>),
}

impl LiveBody {
    /// The span the region's site key is derived from: where the body starts.
    fn span(&self) -> Option<Span> {
        match self {
            Self::Single(body) => body.first().map(Spanned::span),
            Self::Branches(branches) => branches.first().map(|branch| branch.pass.span()),
        }
    }
}

impl Parse for LiveBody {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if !LiveBranch::peek(input) {
            return Ok(Self::Single(input.call(syn::Block::parse_within)?));
        }
        let mut branches: Vec<LiveBranch> = Vec::new();
        while !input.is_empty() {
            let branch: LiveBranch = input.parse()?;
            if branches.iter().any(|seen| seen.pass == branch.pass) {
                return Err(syn::Error::new(
                    branch.pass.span(),
                    format!("duplicate `{}` branch", branch.pass),
                ));
            }
            branches.push(branch);
        }
        Ok(Self::Branches(branches))
    }
}

/// One phase of a `live!` region, written like a `match` arm:
/// `Initial => { ... }` or `Connected => { ... }`.
///
/// The name is emitted as a `Pass` variant, so the compiler resolves it and
/// rejects a name that is not a pass.
pub struct LiveBranch {
    pub pass: Ident,
    pub fat_arrow_token: Token![=>],
    pub body: syn::Block,
    pub comma: Option<Token![,]>,
}

impl LiveBranch {
    /// Whether the input starts with a branch, `Ident => {`.
    fn peek(input: ParseStream) -> bool {
        // `=>` spans two token positions, so the brace sits past `peek3`.
        let fork = input.fork();
        fork.parse::<Ident>().is_ok() && fork.parse::<Token![=>]>().is_ok() && fork.peek(Brace)
    }

    /// The region built from `branches`: one body whose `match` on the
    /// branch to run has an arm per branch.
    ///
    /// One body keeps everything the branches capture moved exactly once.
    /// Every written branch becomes an arm in its own order, so a name that
    /// is not a pass fails to resolve at its own span. The `Connected` arm
    /// runs only in a connected pass of a region that has the branch; a
    /// region without it runs its `Initial` branch in either pass.
    fn expand(
        branches: &[Self],
        cx: &TokenStream,
        borrow_cx: Option<&TokenStream>,
    ) -> TokenStream {
        let has = |name: &str| branches.iter().any(|branch| branch.pass == name);
        let connected = has("Connected");

        let mut arms: Vec<TokenStream> = branches
            .iter()
            .map(|branch| {
                let pass = &branch.pass;
                let stmts = &branch.body.stmts;
                quote! { #topcoat_view::ViewPass::#pass => { #(#stmts)* } }
            })
            .collect();
        if !has("Initial") {
            let span = branches
                .first()
                .map_or_else(Span::call_site, |branch| branch.pass.span());
            let error = quote_spanned! {span=>
                ::core::compile_error!("a `live!` region with branches needs an `Initial` branch");
            };
            arms.push(quote! {
                #topcoat_view::ViewPass::Initial => {
                    #error
                    ::core::unreachable!()
                }
            });
        }
        if !connected {
            arms.push(quote! {
                #topcoat_view::ViewPass::Connected => {
                    ::core::unreachable!("a region without a `Connected` branch runs its `Initial` branch")
                }
            });
        }

        let branch = if connected {
            quote! { __pass }
        } else {
            quote! { #topcoat_view::ViewPass::Initial }
        };
        quote! {{
            let __pass = #topcoat_view::pass(#cx);
            let __branch = #branch;
            #topcoat_view::internal::LiveView::branches(
                __region,
                __pass,
                #connected,
                async move {
                    #borrow_cx
                    match __branch {
                        #(#arms)*
                    }
                },
            )
        }}
    }
}

impl Parse for LiveBranch {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            pass: input.parse()?,
            fat_arrow_token: input.parse()?,
            body: input.parse()?,
            comma: input.parse()?,
        })
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for LiveBranch {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.pass.pretty_print(printer);
        " ".pretty_print(printer);
        self.fat_arrow_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.body.pretty_print(printer);
    }
}

pub struct Emit {
    pub view: View,
}

impl Parse for Emit {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            view: input.parse()?,
        })
    }
}

impl ToTokens for Emit {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut builder = ViewBuilder::new();
        self.view.nodes.lower(&mut builder);
        let owns_cx = self.view.cx.is_some();
        let view = builder.finish().emit_emit(owns_cx);

        let drive = quote! {
            #topcoat_view::internal::LiveView::drive(__region, #view).await
        };

        // The view borrows the context rather than moving it, so the binding
        // has to outlive the drive it is emitted for.
        match &self.view.cx {
            Some(cx) => {
                let cx = &cx.cx;
                quote! {{
                    let __cx: #topcoat_context::Cx = (#cx).clone();
                    #drive
                }}
            }
            None => drive,
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Emit {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.view.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;

    fn emit(source: &str) -> String {
        syn::parse_str::<Emit>(source)
            .unwrap()
            .to_token_stream()
            .to_string()
    }

    #[test]
    fn an_emitted_view_is_driven_into_the_live_view() {
        let tokens = emit("<div></div>");
        assert!(tokens.starts_with(":: topcoat_view :: internal :: LiveView :: drive (__region ,"));
        assert!(tokens.ends_with(". await"), "{tokens}");
    }

    #[test]
    fn an_emitted_view_is_self_contained() {
        let tokens = emit("<div></div>");
        assert!(tokens.contains("ScopeView :: self_contained ("), "{tokens}");
    }

    #[test]
    fn an_emitted_view_keeps_the_ambient_cx() {
        let tokens = emit("<div></div>");
        assert!(!tokens.contains("let __cx"), "{tokens}");
    }

    #[test]
    fn an_explicit_cx_binds_the_context_identifier() {
        let tokens = emit("cx => <div></div>");
        assert!(tokens.contains("Cx = (cx) . clone () ;"), "{tokens}");
        assert!(tokens.contains("let __cx = & __cx ;"), "{tokens}");
    }

    fn live(source: &str) -> String {
        syn::parse_str::<Live>(source)
            .unwrap()
            .to_token_stream()
            .to_string()
    }

    #[test]
    fn a_live_body_is_wrapped_in_an_async_block() {
        let tokens = live("let x = 1; emit! { <div></div> }");
        assert!(tokens.contains("async move { let x = 1 ;"), "{tokens}");
        assert!(tokens.contains("LiveView :: new ("), "{tokens}");
    }

    #[test]
    fn branches_expand_to_one_body_matching_on_the_branch() {
        let tokens =
            live("Initial => { emit! { <div></div> } } Connected => { emit! { <p></p> } }");
        assert!(tokens.contains("LiveView :: branches ("), "{tokens}");
        assert!(tokens.contains("async move { match __branch {"), "{tokens}");
        assert!(tokens.contains(":: ViewPass :: Initial => {"), "{tokens}");
        assert!(tokens.contains(":: ViewPass :: Connected => {"), "{tokens}");
        // With a connected branch, the pass decides which one runs.
        assert!(tokens.contains("let __branch = __pass ;"), "{tokens}");
        assert!(tokens.contains("__pass , true ,"), "{tokens}");
    }

    #[test]
    fn a_region_without_a_connected_branch_runs_its_initial_one_in_either_pass() {
        let tokens = live("Initial => { emit! { <div></div> } }");
        assert!(
            tokens.contains("let __branch = :: topcoat_view :: ViewPass :: Initial ;"),
            "{tokens}"
        );
        assert!(tokens.contains("__pass , false ,"), "{tokens}");
        assert!(
            tokens.contains(":: ViewPass :: Connected => { :: core :: unreachable !"),
            "{tokens}"
        );
    }

    #[test]
    fn a_region_without_an_initial_branch_is_a_compile_error() {
        // The expansion stays intact so the branch names still resolve.
        let tokens = live("Connected => { emit! { <div></div> } }");
        assert!(tokens.contains(":: ViewPass :: Connected => {"), "{tokens}");
        assert!(
            tokens.contains(":: ViewPass :: Initial => { :: core :: compile_error !"),
            "{tokens}"
        );
    }

    #[test]
    fn a_branch_name_is_emitted_as_a_pass_variant() {
        // The compiler reports the unknown variant at the name's span.
        let tokens = live("Initial => { emit! { <div></div> } } Later => { emit! { <p></p> } }");
        assert!(tokens.contains(":: ViewPass :: Later => {"), "{tokens}");
    }

    #[test]
    fn a_duplicate_branch_is_rejected() {
        let error = syn::parse_str::<Live>("Initial => {} Initial => {}")
            .err()
            .expect("the second branch is rejected");
        assert_eq!(error.to_string(), "duplicate `Initial` branch");
    }

    #[test]
    fn a_leading_cx_precedes_the_branches() {
        let tokens = live("cx => Initial => { emit! { <div></div> } }");
        assert!(tokens.contains("Cx = (cx) . clone () ;"), "{tokens}");
        assert!(tokens.contains(":: ViewPass :: Initial =>"), "{tokens}");
        assert!(tokens.contains("let __cx = & __cx ;"), "{tokens}");
    }

    #[cfg(feature = "pretty")]
    #[test]
    fn branches_format_like_match_arms() {
        use topcoat_core_grammar::pretty::{Registry, pretty_print_str};

        let mut registry = Registry::new();
        registry.register_macro::<Live>("live");
        registry.register_macro::<Emit>("emit");
        let formatted = pretty_print_str(
            &registry,
            "live! {Initial => {emit! { <div></div> }}, Connected => {emit! { <p></p> }}}",
        )
        .unwrap();
        assert_eq!(
            formatted,
            "live! {\n    Initial => {\n        emit! { <div></div> }\n    }\n    Connected => {\n        emit! { <p></p> }\n    }\n}"
        );
    }
}
