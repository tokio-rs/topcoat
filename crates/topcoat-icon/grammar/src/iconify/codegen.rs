use std::fmt::Write as _;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Visibility;

use topcoat_core_grammar::paths::{topcoat_icon, topcoat_view};
use topcoat_icon::iconify::{IconSet, ResolvedIcon};

use crate::iconify::{Selection, suggest::did_you_mean};

/// Resolves one icon of `set` for emission. Unknown names are reported at
/// `span` with near-miss suggestions, as are icons that resolve to a
/// transformation, which emission does not support.
pub(crate) fn resolve_icon<'set>(
    set: &'set IconSet,
    name: &str,
    span: Span,
) -> syn::Result<ResolvedIcon<'set>> {
    let prefix = &set.prefix;

    let Some(icon) = set.resolve(name) else {
        let mut message = format!("no icon `{name}` in the Iconify icon set `{prefix}`");
        let candidates = set.icons.keys().chain(set.aliases.keys());
        if let Some(did_you_mean) = did_you_mean(name, candidates.map(String::as_str)) {
            let _ = write!(message, "; {did_you_mean}");
        }
        return Err(syn::Error::new(span, message));
    };

    if !icon.is_untransformed() {
        return Err(syn::Error::new(
            span,
            format!(
                "the icon `{name}` in the Iconify icon set `{prefix}` carries a rotation or \
                 flip, which is not supported"
            ),
        ));
    }

    Ok(icon)
}

/// The const-evaluable `IconData` expression for a resolved icon.
pub(crate) fn icon_expr(icon: &ResolvedIcon<'_>) -> TokenStream {
    let ResolvedIcon {
        body,
        left,
        top,
        width,
        height,
        ..
    } = icon;
    quote! {
        #topcoat_icon::IconData::unescaped_unchecked(
            #topcoat_view::svg::ViewBox::new(#left, #top, #width, #height),
            #body,
        )
    }
}

/// The `const` item for one icon of the selected set.
pub(crate) fn const_item(
    selection: &Selection,
    name: &str,
    icon: &ResolvedIcon<'_>,
    vis: &Visibility,
) -> TokenStream {
    let doc = format!(
        "The `{prefix}:{name}` Iconify icon.",
        prefix = selection.prefix
    );
    let ident = selection.const_ident(name);
    let expr = icon_expr(icon);
    quote! {
        #[doc = #doc]
        #vis const #ident: #topcoat_icon::IconData = #expr;
    }
}

/// The `const` items for every listed name of the selected set: icons and
/// aliases that are not hidden, minus the transformed ones emission does not
/// support. The consts allow `dead_code` because a set is included as a
/// whole, not per used icon.
pub(crate) fn set_consts(selection: &Selection, set: &IconSet, vis: &Visibility) -> TokenStream {
    let names = set.icons.keys().chain(set.aliases.keys());
    let consts = names.filter_map(|name| {
        let icon = set.resolve(name)?;
        (!icon.hidden && icon.is_untransformed()).then(|| {
            let item = const_item(selection, name, &icon, vis);
            quote! {
                #[allow(dead_code)]
                #item
            }
        })
    });
    quote! { #(#consts)* }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_set() -> IconSet {
        serde_json::from_value(serde_json::json!({
            "prefix": "demo",
            "icons": {
                "trash": { "body": "<path d=\"M1 1\"/>" },
                "old-trash": { "body": "<path d=\"M0 0\"/>", "hidden": true },
            },
            "aliases": {
                "bin": { "parent": "trash" },
                "trash-flipped": { "parent": "trash", "hFlip": true },
            },
            "width": 24,
            "height": 24,
        }))
        .unwrap()
    }

    fn selection(source: &str) -> Selection {
        syn::parse_str(&format!("\"{source}\"")).unwrap()
    }

    #[test]
    fn icon_exprs_are_const_iconify_data() {
        let set = demo_set();
        let icon = resolve_icon(&set, "trash", Span::call_site()).unwrap();

        let expected = quote! {
            #topcoat_icon::IconData::unescaped_unchecked(
                #topcoat_view::svg::ViewBox::new(0f32, 0f32, 24f32, 24f32),
                "<path d=\"M1 1\"/>",
            )
        };
        assert_eq!(icon_expr(&icon).to_string(), expected.to_string());
    }

    #[test]
    fn unknown_icons_report_near_misses() {
        let set = demo_set();
        let error = resolve_icon(&set, "trash2", Span::call_site())
            .err()
            .unwrap()
            .to_string();

        assert!(error.contains("no icon `trash2`"), "{error}");
        assert!(error.contains("did you mean `trash`"), "{error}");
    }

    #[test]
    fn transformed_icons_are_unsupported() {
        let set = demo_set();
        let error = resolve_icon(&set, "trash-flipped", Span::call_site())
            .err()
            .unwrap()
            .to_string();

        assert!(error.contains("rotation or flip"), "{error}");
    }

    #[test]
    fn hidden_icons_resolve_by_name() {
        let set = demo_set();
        assert!(resolve_icon(&set, "old-trash", Span::call_site()).is_ok());
    }

    #[test]
    fn set_consts_list_visible_untransformed_names() {
        let set = demo_set();
        let consts = set_consts(&selection("demo"), &set, &Visibility::Inherited).to_string();

        // `trash` and its alias `bin` are emitted; the hidden `old-trash`
        // and the flipped `trash-flipped` are not.
        assert!(consts.contains("const TRASH"), "{consts}");
        assert!(consts.contains("const BIN"), "{consts}");
        assert!(!consts.contains("OLD_TRASH"), "{consts}");
        assert!(!consts.contains("TRASH_FLIPPED"), "{consts}");
    }
}
