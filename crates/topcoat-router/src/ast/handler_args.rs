use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{FnArg, Ident, ItemFn, Type};

pub struct HandlerArgs {
    args: Vec<HandlerArg>,
    request: Option<RequestArg>,
}

struct HandlerArg {
    kind: HandlerArgKind,
}

enum HandlerArgKind {
    Cx,
    Request,
}

pub struct RequestArg {
    pub ty: Type,
}

impl HandlerArgs {
    pub fn parse(item: &ItemFn, kind: &str) -> syn::Result<Self> {
        let mut args = Vec::new();
        let mut has_cx = false;
        let mut request = None;

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
                        if has_cx {
                            return Err(syn::Error::new_spanned(
                                pat_type,
                                format!(
                                    "{kind} functions cannot take more than one `cx: &Cx` parameter"
                                ),
                            ));
                        }
                        has_cx = true;
                        args.push(HandlerArg {
                            kind: HandlerArgKind::Cx,
                        });
                    } else {
                        if request.is_some() {
                            return Err(syn::Error::new_spanned(
                                pat_type,
                                format!(
                                    "{kind} functions cannot take more than one request body parameter"
                                ),
                            ));
                        }
                        request = Some(RequestArg {
                            ty: (*pat_type.ty).clone(),
                        });
                        args.push(HandlerArg {
                            kind: HandlerArgKind::Request,
                        });
                    }
                }
            }
        }

        Ok(Self { args, request })
    }

    pub fn call_args(&self) -> Vec<TokenStream> {
        self.args
            .iter()
            .map(|arg| match arg.kind {
                HandlerArgKind::Cx => quote! { cx },
                HandlerArgKind::Request => {
                    let ident = request_ident();
                    quote! { #ident }
                }
            })
            .collect()
    }

    pub fn request(&self) -> Option<&RequestArg> {
        self.request.as_ref()
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
