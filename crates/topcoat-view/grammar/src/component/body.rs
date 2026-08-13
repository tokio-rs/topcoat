use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Block, Expr, Ident, Pat, Stmt,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
};
use topcoat_core_grammar::paths::topcoat_view;

use crate::view::{
    View,
    hir::{LowerView, ViewBuilder, is_view_macro},
};

/// Rewrites a component body onto the live render.
///
/// The tail `view!` becomes the frame expansion: it builds the view, hands
/// it over through the frame's fill, and keeps driving the frame's live
/// work; a tail that forks (`if`, `match`, a block) gets the expansion in
/// each tail position that is a `view!`. A direct `let x = view! { ... };`
/// statement becomes a pending handle, created at the binding and adopted by
/// the tail expansion. Any other `view!` in the body expands on its own, in
/// blocking mode.
///
/// # Errors
///
/// Returns an error when a bound `view!` has no tail `view!` to adopt it,
/// or when a `view!` body fails to parse.
pub(crate) fn transform(block: &mut Block) -> syn::Result<()> {
    let mut pendings = Vec::new();
    for stmt in &mut block.stmts {
        rewrite_binding(stmt, &mut pendings)?;
    }
    let mut tails = 0;
    rewrite_stmts_tail(block, &pendings, &mut tails)?;
    if !pendings.is_empty() && tails == 0 {
        return Err(syn::Error::new(
            block.span(),
            "a `view!` bound in the body needs the body to end in a `view!` \
             that shows or drops it",
        ));
    }
    let mut returns = ReturnRewriter { error: None };
    returns.visit_block_mut(block);
    match returns.error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Rewrites `return view! { ... };` statements anywhere in the body, so an
/// early return delivers through the fill like the tail does. Nested items,
/// closures, and `async` blocks return somewhere else and are left alone.
struct ReturnRewriter {
    error: Option<syn::Error>,
}

impl VisitMut for ReturnRewriter {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Return(ret) = expr
            && let Some(value) = &mut ret.expr
            && let Expr::Macro(mac) = &**value
            && is_view_macro(&mac.mac.path)
        {
            match syn::parse2::<View>(mac.mac.tokens.clone()) {
                Ok(view) => {
                    let frame = emit_frame(&view, &[], &mac.mac.path);
                    **value = Expr::Verbatim(frame);
                }
                Err(error) => {
                    self.error.get_or_insert(error);
                }
            }
            return;
        }
        visit_mut::visit_expr_mut(self, expr);
    }

    fn visit_item_mut(&mut self, _item: &mut syn::Item) {}

    fn visit_expr_closure_mut(&mut self, _closure: &mut syn::ExprClosure) {}

    fn visit_expr_async_mut(&mut self, _block: &mut syn::ExprAsync) {}
}

/// Rewrites a direct `let x = view! { ... };` statement into a pending
/// handle binding.
fn rewrite_binding(stmt: &mut Stmt, pendings: &mut Vec<Ident>) -> syn::Result<()> {
    let Stmt::Local(local) = stmt else {
        return Ok(());
    };
    let Some(init) = &local.init else {
        return Ok(());
    };
    if init.diverge.is_some() {
        return Ok(());
    }
    let Expr::Macro(mac) = &*init.expr else {
        return Ok(());
    };
    if !is_view_macro(&mac.mac.path) {
        return Ok(());
    }

    let view: View = syn::parse2(mac.mac.tokens.clone())?;
    let frame = emit_frame(&view, &[], &mac.mac.path);
    let pending = format_ident!("__pending{}", pendings.len());

    // A type ascription belongs to the handle, not to the pair the pending
    // form binds.
    let (pat, ascription) = match &local.pat {
        Pat::Type(typed) => ((*typed.pat).clone(), Some((*typed.ty).clone())),
        pat => (pat.clone(), None),
    };
    let init_expr: Expr = syn::parse2(quote! {
        #topcoat_view::internal::pending(|__fill| async { #frame })
    })?;
    let pair = syn::parse::Parser::parse2(
        Pat::parse_multi_with_leading_vert,
        quote! { (#pat, #pending) },
    )?;
    local.pat = match ascription {
        Some(ty) => Pat::Type(syn::PatType {
            attrs: Vec::new(),
            pat: Box::new(pair),
            colon_token: syn::token::Colon::default(),
            ty: syn::parse2(quote! { (#ty, _) })?,
        }),
        None => pair,
    };
    if let Some(init) = &mut local.init {
        *init.expr = init_expr;
    }
    pendings.push(pending);
    Ok(())
}

/// Rewrites every tail position of `expr` that is a `view!` into the frame
/// expansion, descending through blocks, `if` branches, and `match` arms.
fn rewrite_tail(expr: &mut Expr, pendings: &[Ident], tails: &mut usize) -> syn::Result<()> {
    match expr {
        Expr::Macro(mac) if is_view_macro(&mac.mac.path) => {
            let view: View = syn::parse2(mac.mac.tokens.clone())?;
            let frame = emit_frame(&view, pendings, &mac.mac.path);
            *expr = Expr::Verbatim(frame);
            *tails += 1;
        }
        Expr::If(node) => {
            rewrite_block_tail(&mut node.then_branch, pendings, tails)?;
            if let Some((_, else_branch)) = &mut node.else_branch {
                rewrite_tail(else_branch, pendings, tails)?;
            }
        }
        Expr::Match(node) => {
            for arm in &mut node.arms {
                rewrite_tail(&mut arm.body, pendings, tails)?;
            }
        }
        Expr::Block(node) => rewrite_block_tail(&mut node.block, pendings, tails)?,
        Expr::Paren(node) => rewrite_tail(&mut node.expr, pendings, tails)?,
        Expr::Group(node) => rewrite_tail(&mut node.expr, pendings, tails)?,
        _ => {}
    }
    Ok(())
}

fn rewrite_block_tail(block: &mut Block, pendings: &[Ident], tails: &mut usize) -> syn::Result<()> {
    rewrite_stmts_tail(block, pendings, tails)
}

/// Rewrites a block's tail statement, which a bare trailing `view!` parses
/// into as a macro statement rather than an expression.
fn rewrite_stmts_tail(block: &mut Block, pendings: &[Ident], tails: &mut usize) -> syn::Result<()> {
    match block.stmts.last_mut() {
        Some(Stmt::Expr(expr, None)) => rewrite_tail(expr, pendings, tails),
        Some(stmt @ Stmt::Macro(_)) => {
            let Stmt::Macro(mac) = stmt else {
                unreachable!("matched above");
            };
            if mac.semi_token.is_none() && is_view_macro(&mac.mac.path) {
                let view: View = syn::parse2(mac.mac.tokens.clone())?;
                let frame = emit_frame(&view, pendings, &mac.mac.path);
                *stmt = Stmt::Expr(Expr::Verbatim(frame), None);
                *tails += 1;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Emits one live frame for a parsed `view!` body, delivering the built
/// view through `__fill` and adopting the pendings bound above it.
fn emit_frame(view: &View, pendings: &[Ident], mac_path: &syn::Path) -> TokenStream {
    let mut builder = ViewBuilder::new();
    view.nodes.lower(&mut builder);
    let scope = builder.finish();
    let attach = pendings
        .iter()
        .map(|pending| quote! { __refresh.attach(#pending); })
        .collect();
    let delivery = quote! { __fill.fill(__view); };
    let frame = scope.emit_frame_live(&attach, &delivery);
    // The macro invocation itself disappears from the expansion; an
    // underscore use keeps the caller's `view` import counted as used.
    let use_view = (mac_path.segments.len() == 1 && mac_path.leading_colon.is_none())
        .then(|| quote! { #[allow(unused_imports)] use #mac_path as _; });
    let cx = view.cx.as_ref().map(|cx| quote! { #cx });
    if use_view.is_none() && cx.is_none() {
        frame
    } else {
        quote! {{ #use_view #cx #frame }}
    }
}
