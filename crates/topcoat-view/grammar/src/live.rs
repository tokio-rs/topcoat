use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
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
use topcoat_view::ViewPass;

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
        let body = &self.body;
        let connecting = body.connecting();

        quote! {{
            let __region = #topcoat_view::RegionId::new(identity(__cx), #site);
            let __pass = #topcoat_view::pass(__cx);
            #topcoat_view::internal::LiveView::new(
                __region,
                #connecting,
                #body,
            )
        }}
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for Live {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.cx.pretty_print(printer);
        self.body.pretty_print(printer);
    }
}

pub enum LiveBody {
    Single(Vec<syn::Stmt>),
    Branches(Vec<LiveBranch>),
}

impl LiveBody {
    fn connecting(&self) -> bool {
        match self {
            Self::Single(_) => true,
            Self::Branches(branches) => branches.iter().any(LiveBranch::connecting),
        }
    }

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
            branches.push(input.parse()?);
        }
        Ok(Self::Branches(branches))
    }
}

impl ToTokens for LiveBody {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        quote! {
            async move {
                match __pass {

                }
            }
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for LiveBody {
    /// Lays out statements and branches alike on their own lines, like the
    /// statements of a block, keeping standalone comments and single blank
    /// lines between them.
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        let len = match self {
            Self::Single(stmts) => stmts.len(),
            Self::Branches(branches) => branches.len(),
        };
        for index in 0..len {
            match self {
                Self::Single(stmts) => stmts[index].pretty_print(printer),
                Self::Branches(branches) => branches[index].pretty_print(printer),
            }
            if index < len - 1 {
                printer.scan_same_line_trivia();
                printer.scan_force_break();
                printer.scan_break();
                printer.scan_trivia(true, true);
            }
        }
    }
}

pub struct LiveBranch {
    pub pass: Ident,
    pub fat_arrow_token: Token![=>],
    pub body: syn::Block,
    pub comma: Option<Token![,]>,
}

impl LiveBranch {
    fn connecting(&self) -> bool {
        self.pass == format!("{:?}", ViewPass::Connected)
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

impl ParseOption for LiveBranch {
    fn peek(input: ParseStream) -> bool {
        let fork = input.fork();
        fork.parse::<Ident>().is_ok() && fork.parse::<Token![=>]>().is_ok() && fork.peek(Brace)
    }
}

impl ToTokens for LiveBranch {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let pass = &self.pass;
        let body = &self.body;
        quote! { #topcoat_view::ViewPass::#pass => #body }.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for LiveBranch {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.pass.pretty_print(printer);
        " ".pretty_print(printer);
        self.fat_arrow_token.pretty_print(printer);
        " ".pretty_print(printer);
        // The block body is self-delimiting, so the trailing comma is dropped
        // like on a match arm.
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
