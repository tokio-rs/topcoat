use syn::{
    FnArg, GenericArgument, ItemFn, Pat, PathArguments, ReturnType, TraitBoundModifier, Type,
    TypeParamBound,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

/// The parts of a WebSocket shard signature used during expansion.
pub struct WebSocketSignature<'a> {
    pub argument_ident: &'a syn::Ident,
    pub argument_ty: &'a Type,
    pub stream_item_ty: &'a Type,
}

/// The annotated `async fn` that becomes a shard. Validates the function
/// signature shared by HTTP and WebSocket shards: shards must be `async`, must
/// declare a return type, must not take a `self` receiver, and must use
/// identifier patterns for their arguments.
pub struct ShardItem {
    item: ItemFn,
}

impl ShardItem {
    #[must_use]
    pub fn item(&self) -> &ItemFn {
        &self.item
    }

    /// Validates and extracts the WebSocket-specific signature.
    ///
    /// # Errors
    ///
    /// Returns an error if the function does not take exactly one Tokio MPSC
    /// receiver or does not declare an `impl Stream<Item = Output>` return.
    pub fn websocket_signature(&self) -> syn::Result<WebSocketSignature<'_>> {
        let mut cx_count = 0;
        let mut value_argument = None;
        for argument in &self.item.sig.inputs {
            let FnArg::Typed(pat_type) = argument else {
                unreachable!("validated while parsing ShardItem")
            };
            let Pat::Ident(pat_ident) = &*pat_type.pat else {
                unreachable!("validated while parsing ShardItem")
            };
            if pat_ident.ident == "cx" {
                cx_count += 1;
            } else if value_argument.replace((pat_ident, pat_type)).is_some() {
                return Err(syn::Error::new_spanned(
                    &self.item.sig.inputs,
                    "WebSocket shards must take exactly one non-`cx` argument",
                ));
            }
        }

        if cx_count > 1 {
            return Err(syn::Error::new_spanned(
                &self.item.sig.inputs,
                "WebSocket shards may take at most one `cx` argument",
            ));
        }
        let Some((pat_ident, pat_type)) = value_argument else {
            return Err(syn::Error::new_spanned(
                &self.item.sig.inputs,
                "WebSocket shards must take exactly one non-`cx` argument",
            ));
        };
        let argument_ty = receiver_argument_type(&pat_type.ty)?;
        let stream_item_ty = stream_item_type(&self.item.sig.output)?;

        Ok(WebSocketSignature {
            argument_ident: &pat_ident.ident,
            argument_ty,
            stream_item_ty,
        })
    }
}

impl Parse for ShardItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        if item.sig.asyncness.is_none() {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "shards must be async",
            ));
        }
        if let ReturnType::Default = &item.sig.output {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "shards must have a return type",
            ));
        }
        for arg in &item.sig.inputs {
            match arg {
                FnArg::Receiver(receiver) => {
                    return Err(syn::Error::new_spanned(
                        receiver,
                        "shard functions cannot take a `self` receiver",
                    ));
                }
                FnArg::Typed(pat_type) => match &*pat_type.pat {
                    Pat::Ident(_) => {}
                    _ => {
                        return Err(syn::Error::new_spanned(
                            pat_type,
                            "shard function arguments must be identifier patterns",
                        ));
                    }
                },
            }
        }
        Ok(Self { item })
    }
}

fn receiver_argument_type(receiver: &Type) -> syn::Result<&Type> {
    let Type::Path(type_path) = receiver else {
        return Err(receiver_type_error(receiver));
    };
    if type_path.qself.is_some() {
        return Err(receiver_type_error(receiver));
    }

    let segments: Vec<_> = type_path.path.segments.iter().collect();
    if segments.len() != 4
        || segments[0].ident != "tokio"
        || segments[1].ident != "sync"
        || segments[2].ident != "mpsc"
        || segments[3].ident != "Receiver"
        || segments[..3]
            .iter()
            .any(|segment| !matches!(segment.arguments, PathArguments::None))
    {
        return Err(receiver_type_error(receiver));
    }

    let PathArguments::AngleBracketed(arguments) = &segments[3].arguments else {
        return Err(receiver_type_error(receiver));
    };
    if arguments.args.len() != 1 {
        return Err(receiver_type_error(receiver));
    }
    let Some(GenericArgument::Type(argument_ty)) = arguments.args.first() else {
        return Err(receiver_type_error(receiver));
    };
    Ok(argument_ty)
}

fn receiver_type_error(receiver: &Type) -> syn::Error {
    syn::Error::new_spanned(
        receiver,
        "the WebSocket shard argument must be `tokio::sync::mpsc::Receiver<Arg>`",
    )
}

fn stream_item_type(output: &ReturnType) -> syn::Result<&Type> {
    let ReturnType::Type(_, output_ty) = output else {
        unreachable!("validated while parsing ShardItem")
    };
    let Type::ImplTrait(impl_trait) = &**output_ty else {
        return Err(stream_return_error(output_ty));
    };

    let mut item_ty = None;
    for bound in &impl_trait.bounds {
        let TypeParamBound::Trait(trait_bound) = bound else {
            continue;
        };
        if !matches!(trait_bound.modifier, TraitBoundModifier::None) {
            continue;
        }
        let Some(segment) = trait_bound.path.segments.last() else {
            continue;
        };
        if segment.ident != "Stream" {
            continue;
        }
        if item_ty.is_some() {
            return Err(stream_return_error(output_ty));
        }
        let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
            return Err(stream_return_error(output_ty));
        };
        if arguments.args.len() != 1 {
            return Err(stream_return_error(output_ty));
        }
        let Some(GenericArgument::AssocType(item)) = arguments.args.first() else {
            return Err(stream_return_error(output_ty));
        };
        if item.ident != "Item" || item.generics.is_some() {
            return Err(stream_return_error(output_ty));
        }
        item_ty = Some(&item.ty);
    }

    item_ty.ok_or_else(|| stream_return_error(output_ty))
}

fn stream_return_error(output: &Type) -> syn::Error {
    syn::Error::new_spanned(
        output,
        "WebSocket shards must return `impl Stream<Item = Output>`",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<ShardItem>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    fn websocket_err(source: &str) -> String {
        let item = syn::parse_str::<ShardItem>(source).unwrap();
        match item.websocket_signature() {
            Ok(_) => panic!("expected WebSocket signature error for `{source}`"),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn accepts_async_fn_with_return_type() {
        syn::parse_str::<ShardItem>("async fn counter(cx: &Cx) -> Result {}").unwrap();
    }

    #[test]
    fn rejects_non_async_fn() {
        assert!(parse_err("fn counter() -> Result {}").contains("shards must be async"));
    }

    #[test]
    fn rejects_missing_return_type() {
        assert!(parse_err("async fn counter() {}").contains("must have a return type"));
    }

    #[test]
    fn rejects_self_receiver() {
        let err = parse_err("async fn counter(&self) -> Result {}");
        assert!(err.contains("cannot take a `self` receiver"));
    }

    #[test]
    fn rejects_non_ident_pattern() {
        let err = parse_err("async fn counter((a, b): (u8, u8)) -> Result {}");
        assert!(err.contains("must be identifier patterns"));
    }

    #[test]
    fn accepts_websocket_signature_with_optional_cx() {
        for source in [
            "async fn events(values: tokio::sync::mpsc::Receiver<String>) -> impl futures_core::Stream<Item = Result> {}",
            "async fn events(cx: &Cx, values: ::tokio::sync::mpsc::Receiver<String>) -> impl Stream<Item = Result> + Send {}",
        ] {
            let item = syn::parse_str::<ShardItem>(source).unwrap();
            let signature = item.websocket_signature().unwrap();
            assert_eq!(signature.argument_ident, "values");
            assert!(matches!(signature.argument_ty, Type::Path(_)));
            assert!(matches!(signature.stream_item_ty, Type::Path(_)));
        }
    }

    #[test]
    fn rejects_wrong_websocket_argument_count() {
        assert!(
            websocket_err("async fn events() -> impl Stream<Item = Result> {}")
                .contains("exactly one")
        );
        assert!(
            websocket_err(
                "async fn events(a: tokio::sync::mpsc::Receiver<String>, b: tokio::sync::mpsc::Receiver<String>) -> impl Stream<Item = Result> {}"
            )
            .contains("exactly one")
        );
    }

    #[test]
    fn rejects_wrong_receiver_syntax() {
        for source in [
            "async fn events(values: Receiver<String>) -> impl Stream<Item = Result> {}",
            "async fn events(values: other::sync::mpsc::Receiver<String>) -> impl Stream<Item = Result> {}",
            "async fn events(values: tokio::sync::mpsc::Sender<String>) -> impl Stream<Item = Result> {}",
            "async fn events(values: tokio::sync::mpsc::Receiver<String, String>) -> impl Stream<Item = Result> {}",
        ] {
            assert!(
                websocket_err(source).contains("tokio::sync::mpsc::Receiver<Arg>"),
                "{source}"
            );
        }
    }

    #[test]
    fn rejects_wrong_stream_syntax() {
        for source in [
            "async fn events(values: tokio::sync::mpsc::Receiver<String>) -> Result {}",
            "async fn events(values: tokio::sync::mpsc::Receiver<String>) -> impl Iterator<Item = Result> {}",
            "async fn events(values: tokio::sync::mpsc::Receiver<String>) -> impl Stream {}",
            "async fn events(values: tokio::sync::mpsc::Receiver<String>) -> impl Stream<Result> {}",
        ] {
            assert!(
                websocket_err(source).contains("impl Stream<Item = Output>"),
                "{source}"
            );
        }
    }
}
