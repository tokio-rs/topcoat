#![cfg_attr(docsrs, feature(doc_cfg))]

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{ToTokens as _, format_ident, quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use topcoat_core_grammar::{
    ParseOption,
    paths::{topcoat_error, topcoat_shell_view, topcoat_view_macro},
};
use topcoat_view_grammar::{
    leading_cx::LeadingCx,
    view::{Component, Nodes},
};

/// The parsed body of a `shell_view!` invocation.
pub struct ShellView {
    /// The optional request context binding supplied by `cx =>`.
    pub cx: Option<LeadingCx>,
    /// The shell markup, including any inline `defer` nodes.
    pub body: TokenStream,
}

impl ShellView {
    /// Expands the shell and its inline deferred components.
    ///
    /// # Errors
    ///
    /// Returns an error when a `defer` node does not contain a valid component
    /// invocation and placeholder view.
    pub fn expand(&self) -> syn::Result<TokenStream> {
        let (body, deferred) = rewrite(self.body.clone())?;
        let builder = format_ident!("__shell_view_builder", span = Span::mixed_site());
        let shell = format_ident!("__shell_view_shell", span = Span::mixed_site());
        let ambient_cx = format_ident!("__cx", span = Span::call_site());
        let cx = self.cx.as_ref().map_or(&ambient_cx, |leading| &leading.cx);
        let leading_cx = &self.cx;
        let bindings = deferred.iter().enumerate().map(|(index, deferred)| {
            let slot = slot_ident(index);
            let owned_cx = format_ident!("__shell_view_owned_cx", span = Span::mixed_site());
            let deferred_cx = format_ident!("__shell_view_deferred_cx", span = Span::mixed_site());
            let component = &deferred.component;
            let placeholder = &deferred.placeholder;

            quote_spanned! {deferred.span=>
                let #slot = #builder.defer(
                    #topcoat_view_macro::view! { #leading_cx #placeholder }?,
                    move |#owned_cx| async move {
                        let #deferred_cx = #owned_cx.as_ref();
                        #topcoat_view_macro::view! { #deferred_cx => #component }
                    },
                );
            }
        });

        Ok(quote! {
            async {
                let mut #builder = #topcoat_shell_view::ShellView::builder(#cx);
                #(#bindings)*
                let #shell = #topcoat_view_macro::view! { #leading_cx #body }?;
                ::core::result::Result::<
                    #topcoat_shell_view::ShellView,
                    #topcoat_error::Error,
                >::Ok(#builder.finish(#shell))
            }
            .await
        })
    }
}

impl Parse for ShellView {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            cx: input.call(LeadingCx::parse_option)?,
            body: input.parse()?,
        })
    }
}

struct DeferredRender {
    span: Span,
    component: TokenStream,
    placeholder: TokenStream,
}

fn rewrite(body: TokenStream) -> syn::Result<(TokenStream, Vec<DeferredRender>)> {
    let trees = body.into_iter().collect::<Vec<_>>();
    let mut rewritten = TokenStream::new();
    let mut deferred = Vec::new();
    let mut index = 0;

    while index < trees.len() {
        let Some(ident) = ident_named(&trees[index], "defer") else {
            rewritten.extend([trees[index].clone()]);
            index += 1;
            continue;
        };
        let Some(paren_index) = trees[index + 1..]
            .iter()
            .position(|tree| matches!(tree, TokenTree::Group(group) if group.delimiter() == Delimiter::Parenthesis))
            .map(|offset| index + 1 + offset)
        else {
            rewritten.extend([trees[index].clone()]);
            index += 1;
            continue;
        };
        if paren_index == index + 1 {
            return Err(syn::Error::new(
                ident.span(),
                "expected a component after `defer`",
            ));
        }
        let component = trees[index + 1..=paren_index]
            .iter()
            .cloned()
            .collect::<TokenStream>();
        if syn::parse2::<Component>(component.clone()).is_err() {
            rewritten.extend([trees[index].clone()]);
            index += 1;
            continue;
        }
        let Some(TokenTree::Group(placeholder)) = trees.get(paren_index + 1) else {
            return Err(syn::Error::new(
                ident.span(),
                "expected a placeholder block after the deferred component",
            ));
        };
        if placeholder.delimiter() != Delimiter::Brace {
            return Err(syn::Error::new(
                placeholder.span(),
                "expected a placeholder block after the deferred component",
            ));
        }

        syn::parse2::<Nodes>(placeholder.stream())?;

        let slot = slot_ident(deferred.len());
        quote_spanned! {ident.span()=> (#slot)}.to_tokens(&mut rewritten);
        deferred.push(DeferredRender {
            span: ident.span(),
            component,
            placeholder: placeholder.stream(),
        });
        index = paren_index + 2;
    }

    Ok((rewritten, deferred))
}

fn ident_named<'a>(tree: &'a TokenTree, name: &str) -> Option<&'a proc_macro2::Ident> {
    match tree {
        TokenTree::Ident(ident) if ident == name => Some(ident),
        _ => None,
    }
}

fn slot_ident(index: usize) -> proc_macro2::Ident {
    format_ident!("__shell_view_deferred_{index}", span = Span::mixed_site())
}

#[cfg(test)]
mod tests {
    use quote::ToTokens as _;

    use super::*;

    fn expand(source: &str) -> String {
        syn::parse_str::<ShellView>(source)
            .unwrap()
            .expand()
            .unwrap()
            .to_string()
    }

    #[test]
    fn expands_inline_defer_inside_markup() {
        let output = expand(
            r#"cx => <main><section>defer newsfeed(limit: 3) { <p>"Loading"</p> }</section></main>"#,
        );

        assert!(output.contains("ShellView :: builder (cx)"));
        assert!(output.contains(". defer"));
        assert!(output.contains("newsfeed (limit : 3)"));
        assert!(output.contains("Loading"));
        assert!(output.contains("__shell_view_deferred_0"));
    }

    #[test]
    fn expands_multiple_deferred_components() {
        let output = expand(r#"defer first() { "One" } <div>defer second() { "Two" }</div>"#);

        assert!(output.contains("__shell_view_deferred_0"));
        assert!(output.contains("__shell_view_deferred_1"));
    }

    #[test]
    fn rejects_a_missing_placeholder() {
        let shell = syn::parse_str::<ShellView>("defer newsfeed()").unwrap();
        assert!(
            shell
                .expand()
                .unwrap_err()
                .to_string()
                .contains("expected a placeholder block")
        );
    }

    #[test]
    fn retains_plain_view_tokens() {
        let shell = syn::parse_str::<ShellView>(r#"cx => <p>"Hello"</p>"#).unwrap();
        assert_eq!(
            shell.body.to_token_stream().to_string(),
            r#"< p > "Hello" </ p >"#
        );
    }
}
