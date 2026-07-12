use std::collections::HashSet;

use proc_macro2::{Ident, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Block, ExprClosure, ExprForLoop, ExprIf, ExprMacro, ExprPath, ExprWhile, LitStr, Macro, Pat,
    Stmt, StmtMacro, Token,
    parse::{Parse, ParseStream},
    visit::{self, Visit},
};

use topcoat_core_grammar::paths::topcoat_runtime;

use crate::expr::{
    Expr,
    name_resolver::{NameResolver, ResolvedIdent},
};

impl Expr {
    pub(super) fn expr_macro(
        expr: &ExprMacro,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        BuiltinMacro::parse(&expr.mac)?.lower(rust, js, names)
    }

    pub(super) fn stmt_macro(
        stmt_macro: &StmtMacro,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        BuiltinMacro::parse(&stmt_macro.mac)?.lower(rust, js, names)
    }
}

enum BuiltinMacro {
    Raw(RawMacro),
}

impl BuiltinMacro {
    fn parse(mac: &Macro) -> syn::Result<Self> {
        let ident = mac.path.get_ident().ok_or_else(|| {
            syn::Error::new_spanned(&mac.path, "only single-identifier macros are supported")
        })?;

        match ident.to_string().as_str() {
            "raw" => Ok(Self::Raw(syn::parse2(mac.tokens.clone())?)),
            _ => Err(syn::Error::new_spanned(
                &mac.path,
                "unsupported expression macro",
            )),
        }
    }

    fn lower(
        &self,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        match self {
            Self::Raw(raw) => raw.lower(rust, js, names),
        }
    }
}

struct RawMacro {
    js: LitStr,
    rust: Option<syn::Expr>,
}

impl Parse for RawMacro {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let js = input.parse()?;
        let rust = if input.is_empty() {
            None
        } else {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                return Err(input.error("expected Rust expression after comma"));
            }

            let rust = input.parse()?;
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
                if !input.is_empty() {
                    return Err(input.error("expected end of raw! arguments"));
                }
            }
            Some(rust)
        };

        Ok(Self { js, rust })
    }
}

impl RawMacro {
    fn lower(
        &self,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        js.push_str(&self.interpolate_js(names)?);

        match &self.rust {
            Some(rust_expr) => {
                let locals = RawRustLocalCollector::collect(rust_expr, names);
                // Bound to a local because it is interpolated inside the
                // `#(...)*` repetition below, where a bare `#topcoat_runtime`
                // would expand to a `let` binding that cannot shadow the
                // imported constant.
                let into_real = quote!(#topcoat_runtime::Surrogate::into_real);
                quote! {{
                    #(
                        let #locals = #into_real(#locals);
                    )*
                    #topcoat_runtime::Surrogated::into_surrogate(#rust_expr)
                }}
                .to_tokens(rust);
            }
            None => {
                quote! {
                    ::core::panic!("raw! needs a Rust expression to run on the server")
                }
                .to_tokens(rust);
            }
        }

        Ok(())
    }

    fn interpolate_js(&self, names: &mut NameResolver) -> syn::Result<String> {
        let input = self.js.value();
        let mut output = String::new();
        let mut rest = input.as_str();

        while let Some(start) = rest.find("${") {
            output.push_str(&rest[..start]);

            let interpolation = &rest[start + 2..];
            let Some(end) = interpolation.find('}') else {
                return Err(syn::Error::new(
                    self.js.span(),
                    "unterminated raw! interpolation",
                ));
            };

            let ident = syn::parse_str::<Ident>(&interpolation[..end]).map_err(|_| {
                syn::Error::new(self.js.span(), "raw! interpolation must be `${ident}`")
            })?;

            let js_name = match names.resolve(&ident) {
                ResolvedIdent::Local { js_name, .. } | ResolvedIdent::External { js_name, .. } => {
                    js_name
                }
            };
            output.push_str(&js_name);

            rest = &interpolation[end + 1..];
        }

        output.push_str(rest);
        Ok(output)
    }
}

struct RawRustLocalCollector<'a> {
    names: &'a NameResolver,
    scopes: Vec<HashSet<String>>,
    seen: HashSet<String>,
    locals: Vec<Ident>,
}

impl<'a> RawRustLocalCollector<'a> {
    fn collect(expr: &syn::Expr, names: &'a NameResolver) -> Vec<Ident> {
        let mut collector = Self {
            names,
            scopes: Vec::new(),
            seen: HashSet::new(),
            locals: Vec::new(),
        };
        collector.visit_expr(expr);
        collector.locals
    }

    fn is_shadowed(&self, ident: &Ident) -> bool {
        let name = ident.to_string();
        self.scopes.iter().rev().any(|scope| scope.contains(&name))
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashSet::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn bind_pat(&mut self, pat: &Pat) {
        let mut collector = PatternIdentCollector::default();
        collector.visit_pat(pat);

        if let Some(scope) = self.scopes.last_mut() {
            for ident in collector.idents {
                scope.insert(ident.to_string());
            }
        }
    }
}

impl<'ast> Visit<'ast> for RawRustLocalCollector<'_> {
    fn visit_expr_path(&mut self, expr: &'ast ExprPath) {
        if expr.qself.is_none()
            && let Some(ident) = expr.path.get_ident()
        {
            let name = ident.to_string();
            if !self.is_shadowed(ident)
                && self.names.is_surrogate_local(ident)
                && self.seen.insert(name)
            {
                self.locals.push(ident.clone());
            }
        }

        visit::visit_expr_path(self, expr);
    }

    fn visit_block(&mut self, block: &'ast Block) {
        self.push_scope();
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
        self.pop_scope();
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        match stmt {
            Stmt::Local(local) => {
                if let Some(init) = &local.init {
                    self.visit_expr(&init.expr);
                    if let Some((_, diverge)) = &init.diverge {
                        self.visit_expr(diverge);
                    }
                }
                self.bind_pat(&local.pat);
            }
            other => visit::visit_stmt(self, other),
        }
    }

    fn visit_expr_closure(&mut self, closure: &'ast ExprClosure) {
        self.push_scope();
        for input in &closure.inputs {
            self.bind_pat(input);
        }
        self.visit_expr(&closure.body);
        self.pop_scope();
    }

    fn visit_expr_for_loop(&mut self, expr: &'ast ExprForLoop) {
        self.visit_expr(&expr.expr);
        self.push_scope();
        self.bind_pat(&expr.pat);
        self.visit_block(&expr.body);
        self.pop_scope();
    }

    fn visit_expr_if(&mut self, expr: &'ast ExprIf) {
        match &*expr.cond {
            syn::Expr::Let(condition) => {
                self.visit_expr(&condition.expr);
                self.push_scope();
                self.bind_pat(&condition.pat);
                self.visit_block(&expr.then_branch);
                self.pop_scope();
            }
            condition => {
                self.visit_expr(condition);
                self.visit_block(&expr.then_branch);
            }
        }

        if let Some((_, else_branch)) = &expr.else_branch {
            self.visit_expr(else_branch);
        }
    }

    fn visit_expr_while(&mut self, expr: &'ast ExprWhile) {
        match &*expr.cond {
            syn::Expr::Let(condition) => {
                self.visit_expr(&condition.expr);
                self.push_scope();
                self.bind_pat(&condition.pat);
                self.visit_block(&expr.body);
                self.pop_scope();
            }
            condition => {
                self.visit_expr(condition);
                self.visit_block(&expr.body);
            }
        }
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.push_scope();
        self.bind_pat(&arm.pat);
        if let Some((_, guard)) = &arm.guard {
            self.visit_expr(guard);
        }
        self.visit_expr(&arm.body);
        self.pop_scope();
    }
}

#[derive(Default)]
struct PatternIdentCollector {
    idents: Vec<Ident>,
}

impl<'ast> Visit<'ast> for PatternIdentCollector {
    fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
        self.idents.push(pat.ident.clone());
        visit::visit_pat_ident(self, pat);
    }
}
