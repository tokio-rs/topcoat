use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{parse_quote, spanned::Spanned};
use topcoat_core_grammar::paths::topcoat_runtime;

pub(super) fn expand(source: &syn::Expr) -> syn::Result<TokenStream> {
    let span = source.span();
    let location = span.start();
    let text = source.to_token_stream().to_string();
    let hash = text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    let id = format!(
        "{}:{}:{}:{hash:016x}",
        span.file(),
        location.line,
        location.column
    );
    let handler = matches!(source, syn::Expr::Closure(_));
    let metadata = serde_json::json!({"id": id, "handler": handler, "source": text}).to_string();
    let mut js = quote!(#topcoat_runtime::Js::source("undefined"));
    if let Ok(path) = std::env::var("TOPCOAT_WASM_MANIFEST") {
        let manifest: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).map_err(|error| {
                syn::Error::new(span, format!("cannot read Wasm manifest {path}: {error}"))
            })?)
            .map_err(|error| syn::Error::new(span, error))?;
        let entry = manifest["expressions"].get(&id).ok_or_else(|| {
            syn::Error::new(
                span,
                "expression missing from Wasm analysis; rebuild the client bundles",
            )
        })?;
        let prefix = format!(
            "globalThis.__topcoatWasm(cx,{},[",
            serde_json::to_string(&id).unwrap()
        );
        js = quote!(#topcoat_runtime::Js::builder().source(#prefix));
        for (index, capture) in entry["captures"]
            .as_array()
            .ok_or_else(|| syn::Error::new(span, "invalid Wasm capture manifest"))?
            .iter()
            .enumerate()
        {
            if index != 0 {
                js = quote!(#js.source(","));
            }
            let name: syn::Ident = syn::parse_str(
                capture["name"]
                    .as_str()
                    .ok_or_else(|| syn::Error::new(span, "capture name missing"))?,
            )?;
            let name = syn::Ident::new(&name.to_string(), Span::call_site());
            js = if capture["signal"].as_bool() == Some(true) {
                quote!(#js.surrogate(&#topcoat_runtime::__wasm_signal_capture(&#name)))
            } else {
                quote!(#js.surrogate(&#topcoat_runtime::__wasm_value(&#name)))
            };
        }
        js = quote!(#js.source("])").build());
    } else if std::env::var("TOPCOAT_WASM_ANALYSIS").as_deref() != Ok("1") {
        return Err(syn::Error::new(
            span,
            "Wasm expressions require a generated manifest; use the Topcoat Wasm build tool",
        ));
    }
    if let syn::Expr::Closure(closure) = source {
        if closure.asyncness.is_some() || closure.inputs.len() != 1 {
            return Err(syn::Error::new(
                span,
                "Wasm event handlers currently require one synchronous event argument",
            ));
        }
        let mut closure = closure.clone();
        let parameter = closure.inputs.first().unwrap();
        if !matches!(parameter, syn::Pat::Type(_)) {
            closure.inputs[0] = syn::Pat::Type(syn::PatType {
                attrs: Vec::new(),
                pat: Box::new(parameter.clone()),
                colon_token: syn::token::Colon::default(),
                ty: Box::new(parse_quote!(#topcoat_runtime::Event)),
            });
        }
        Ok(quote! {{
            let __js = #js;
            let _ = #topcoat_runtime::__topcoat_wasm_root(#metadata, #closure);
            #topcoat_runtime::Expr::evaluate(|| #topcoat_runtime::__wasm_event as fn(#topcoat_runtime::Event), __js)
        }})
    } else {
        Ok(quote! {{
            let __js = #js;
            #topcoat_runtime::Expr::evaluate(
                #topcoat_runtime::__topcoat_wasm_root(#metadata, || #source), __js)
        }})
    }
}
