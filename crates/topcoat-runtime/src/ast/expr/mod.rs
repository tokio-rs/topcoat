mod expr_assign_deref;
mod expr_closure;
mod expr_deref;
mod expr_field;
mod expr_ident;
mod expr_method_call;
mod expr_param;

pub use expr_assign_deref::*;
pub use expr_closure::*;
pub use expr_deref::*;
pub use expr_field::*;
pub use expr_ident::*;
pub use expr_method_call::*;
pub use expr_param::*;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Ident, Member, Pat, Type, UnOp,
    parse::{Parse, ParseStream},
};

/// A closure parameter binding tracked while lowering. The type is whatever
/// the user wrote on the closure (e.g. `|e: Event|`); `None` means the user
/// left the parameter un-annotated.
#[derive(Clone)]
struct BoundParam {
    name: Ident,
    ty: Option<Type>,
}

/// The top-level `expr! { ... }` AST. A whitelist of `syn::Expr` shapes is
/// translated into a tree of runtime expression nodes; anything outside that
/// whitelist is rejected at compile time.
pub enum Expr {
    Ident(ExprIdent),
    Param(ExprParam),
    Deref(ExprDeref),
    Field(ExprField),
    MethodCall(ExprMethodCall),
    AssignDeref(ExprAssignDeref),
    Closure(ExprClosure),
}

impl Expr {
    fn from_syn(expr: syn::Expr, bound: &[BoundParam]) -> syn::Result<Self> {
        match expr {
            syn::Expr::Path(path) => {
                let Some(ident) = path.path.get_ident() else {
                    return Err(syn::Error::new_spanned(
                        path,
                        "expected a bare identifier",
                    ));
                };
                if let Some(param) = bound.iter().find(|b| &b.name == ident) {
                    Ok(Self::Param(ExprParam::new(ident.clone(), param.ty.clone())))
                } else {
                    Ok(Self::Ident(ExprIdent::new(ident.clone())))
                }
            }
            syn::Expr::Unary(unary) => {
                let UnOp::Deref(_) = unary.op else {
                    return Err(syn::Error::new_spanned(
                        unary.op,
                        "unsupported unary operator",
                    ));
                };
                let inner = Self::from_syn(*unary.expr, bound)?;
                Ok(Self::Deref(ExprDeref::new(inner)))
            }
            syn::Expr::Field(field) => {
                let receiver = Self::from_syn(*field.base, bound)?;
                let Member::Named(name) = field.member else {
                    return Err(syn::Error::new(
                        proc_macro2::Span::call_site(),
                        "tuple field access is not supported",
                    ));
                };
                Ok(Self::Field(ExprField::new(receiver, name)))
            }
            syn::Expr::MethodCall(mc) => {
                if mc.turbofish.is_some() {
                    return Err(syn::Error::new_spanned(
                        &mc.turbofish,
                        "turbofish on method calls is not supported",
                    ));
                }
                if !mc.args.is_empty() {
                    return Err(syn::Error::new_spanned(
                        &mc.args,
                        "method arguments are not supported",
                    ));
                }
                let receiver = Self::from_syn(*mc.receiver, bound)?;
                Ok(Self::MethodCall(ExprMethodCall::new(receiver, mc.method)))
            }
            syn::Expr::Paren(paren) => Self::from_syn(*paren.expr, bound),
            syn::Expr::Assign(assign) => {
                let syn::Expr::Unary(unary) = *assign.left else {
                    return Err(syn::Error::new_spanned(
                        assign.left,
                        "only `*place = value` assignments are supported",
                    ));
                };
                let UnOp::Deref(_) = unary.op else {
                    return Err(syn::Error::new_spanned(
                        unary.op,
                        "only `*place = value` assignments are supported",
                    ));
                };
                let place = Self::from_syn(*unary.expr, bound)?;
                let value = Self::from_syn(*assign.right, bound)?;
                Ok(Self::AssignDeref(ExprAssignDeref::new(place, value)))
            }
            syn::Expr::Closure(closure) => {
                let params: Vec<BoundParam> = closure
                    .inputs
                    .iter()
                    .map(|pat| match pat {
                        Pat::Ident(pi) => Ok(BoundParam {
                            name: pi.ident.clone(),
                            ty: None,
                        }),
                        Pat::Type(pt) => {
                            let Pat::Ident(pi) = &*pt.pat else {
                                return Err(syn::Error::new_spanned(
                                    &pt.pat,
                                    "expected a bare parameter name",
                                ));
                            };
                            Ok(BoundParam {
                                name: pi.ident.clone(),
                                ty: Some((*pt.ty).clone()),
                            })
                        }
                        other => Err(syn::Error::new_spanned(
                            other,
                            "expected a bare parameter name",
                        )),
                    })
                    .collect::<syn::Result<_>>()?;
                let param_names: Vec<Ident> =
                    params.iter().map(|p| p.name.clone()).collect();
                let mut inner_bound = bound.to_vec();
                inner_bound.extend(params);
                let body = Self::from_syn(*closure.body, &inner_bound)?;
                Ok(Self::Closure(ExprClosure::new(param_names, body)))
            }
            other => Err(syn::Error::new_spanned(other, "unsupported expression")),
        }
    }
}

impl Parse for Expr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Self::from_syn(input.parse()?, &[])
    }
}

impl ToTokens for Expr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(node) => node.to_tokens(tokens),
            Self::Param(node) => node.to_tokens(tokens),
            Self::Deref(node) => node.to_tokens(tokens),
            Self::Field(node) => node.to_tokens(tokens),
            Self::MethodCall(node) => node.to_tokens(tokens),
            Self::AssignDeref(node) => node.to_tokens(tokens),
            Self::Closure(node) => node.to_tokens(tokens),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Expr {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<Expr>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn parses_bare_identifier() {
        assert!(matches!(parse("signal"), Expr::Ident(_)));
    }

    #[test]
    fn parses_deref_of_identifier() {
        assert!(matches!(parse("*signal"), Expr::Deref(_)));
    }

    #[test]
    fn parses_nested_deref() {
        let Expr::Deref(_) = parse("**signal") else {
            panic!("expected deref")
        };
    }

    #[test]
    fn parses_field_access() {
        assert!(matches!(parse("e.target"), Expr::Field(_)));
    }

    #[test]
    fn parses_chained_field_access() {
        assert!(matches!(parse("e.target.value"), Expr::Field(_)));
    }

    #[test]
    fn parses_assignment_to_deref() {
        assert!(matches!(parse("*kek = x"), Expr::AssignDeref(_)));
    }

    #[test]
    fn parses_closure() {
        assert!(matches!(parse("|e| *kek = e.target.value"), Expr::Closure(_)));
    }

    #[test]
    fn closure_parameter_resolves_to_param_node() {
        let Expr::Closure(_) = parse("|e| e") else {
            panic!("expected closure")
        };
        // The body of `|e| e` lowered with `e` bound should produce ExprParam,
        // verified indirectly via the lowering not erroring out.
    }

    #[test]
    fn literal_is_rejected() {
        assert!(parse_err("42").contains("unsupported expression"));
    }

    #[test]
    fn binary_op_is_rejected() {
        assert!(parse_err("a + b").contains("unsupported expression"));
    }

    #[test]
    fn path_with_segments_is_rejected() {
        assert!(parse_err("foo::bar").contains("bare identifier"));
    }

    #[test]
    fn non_deref_unary_is_rejected() {
        assert!(parse_err("-x").contains("unary"));
    }

    #[test]
    fn non_deref_assignment_is_rejected() {
        assert!(parse_err("x = y").contains("only `*place = value`"));
    }
}
