use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FnArg, Ident, ItemFn, Type};

pub struct HandlerArgs {
    args: Vec<HandlerArg>,
}

/// A handler function parameter, classified by role.
pub enum HandlerArg {
    /// The `cx: &Cx` request context parameter.
    Cx,
    /// The request body parameter, with its declared type.
    Request(Box<Type>),
}

impl HandlerArgs {
    pub fn parse(item: &ItemFn, kind: &str) -> syn::Result<Self> {
        let mut args: Vec<HandlerArg> = Vec::new();

        for arg in &item.sig.inputs {
            match arg {
                FnArg::Receiver(receiver) => {
                    return Err(syn::Error::new_spanned(
                        receiver,
                        format!("{kind} functions cannot take a `self` receiver"),
                    ));
                }
                FnArg::Typed(pat_type) => {
                    if is_cx(&pat_type.ty) {
                        if args.iter().any(|arg| matches!(arg, HandlerArg::Cx)) {
                            return Err(syn::Error::new_spanned(
                                pat_type,
                                format!(
                                    "{kind} functions cannot take more than one `cx: &Cx` parameter"
                                ),
                            ));
                        }
                        args.push(HandlerArg::Cx);
                    } else {
                        if args.iter().any(|arg| matches!(arg, HandlerArg::Request(_))) {
                            return Err(syn::Error::new_spanned(
                                pat_type,
                                format!(
                                    "{kind} functions cannot take more than one request body parameter"
                                ),
                            ));
                        }
                        args.push(HandlerArg::Request(pat_type.ty.clone()));
                    }
                }
            }
        }

        Ok(Self { args })
    }

    /// The handler's parameters in declaration order.
    pub fn iter(&self) -> impl Iterator<Item = &HandlerArg> {
        self.args.iter()
    }

    /// The declared type of the request body parameter, if any.
    pub fn request(&self) -> Option<&Type> {
        self.args.iter().find_map(|arg| match arg {
            HandlerArg::Request(ty) => Some(&**ty),
            HandlerArg::Cx => None,
        })
    }

    pub fn call_args(&self) -> Vec<TokenStream> {
        self.args
            .iter()
            .map(|arg| match arg {
                HandlerArg::Cx => quote! { cx },
                HandlerArg::Request(_) => {
                    let ident = request_ident();
                    quote! { #ident }
                }
            })
            .collect()
    }
}

pub fn request_ident() -> Ident {
    Ident::new("__topcoat_request", Span::mixed_site())
}

fn is_cx(ty: &Type) -> bool {
    let Type::Reference(reference) = ty else {
        return false;
    };
    if reference.mutability.is_some() {
        return false;
    }

    let Type::Path(path) = &*reference.elem else {
        return false;
    };

    path.qself.is_none()
        && path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "Cx")
}
