use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::Path;
use topcoat_core_grammar::paths::{topcoat_error, topcoat_view};

use crate::view::{NamedArg, hir::Scope};

/// A component invocation, emitted through the props builder.
pub(crate) struct Component {
    pub path: Path,
    pub named_args: Vec<NamedArg>,
    pub children: Option<Scope>,
    pub span: Span,
}

impl Component {
    pub(in crate::view::hir) fn emit_future(&self) -> TokenStream {
        let Self {
            path,
            named_args,
            children,
            span,
        } = self;
        let setters = named_args.iter().map(|arg| {
            let ident = &arg.ident;
            let value = &arg.value;
            quote! { .#ident(#value) }
        });

        if let Some(children) = children {
            let child = children.emit_future();
            quote_spanned! {*span=>
                async {
                    use #topcoat_view::Component;
                    let __child_slot = __ViewSlot::new();
                    let props = #path::props_builder()
                        #(#setters)*
                        .child(__child_slot.view())
                        .build();
                    let __child = async {
                        __child_slot.fill((#child).await?);
                        ::core::result::Result::<(), #topcoat_error::Error>::Ok(())
                    };
                    // The marker is built via `Default` so the same construction
                    // works for unit-struct and generic (`PhantomData`) markers.
                    #[allow(clippy::default_constructed_unit_structs)]
                    let __component = Component::render(#path::default(), __cx, props);
                    let (__child_result, __component_result) =
                        __join(__child, __component).await;
                    __child_result?;
                    __component_result
                }
            }
        } else {
            quote_spanned! {*span=>
                async {
                    use #topcoat_view::Component;
                    let props = #path::props_builder()#(#setters)*.build();
                    // The marker is built via `Default` so the same construction
                    // works for unit-struct and generic (`PhantomData`) markers.
                    #[allow(clippy::default_constructed_unit_structs)]
                    Component::render(#path::default(), __cx, props).await
                }
            }
        }
    }
}
