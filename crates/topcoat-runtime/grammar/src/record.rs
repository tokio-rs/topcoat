use proc_macro2::{Group, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::{
    Fields, Ident, ItemStruct,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use topcoat_core_grammar::paths::topcoat_runtime;

/// Names reserved for browser runtime members and JavaScript protocols.
const RESERVED_FIELDS: &[&str] = &[
    "__proto__",
    "clone",
    "constructor",
    "dehydrate",
    "deref",
    "deref_mut",
    "then",
    "toJSON",
    "toString",
    "valueOf",
];

/// Arguments to `#[record]`, which accepts none.
pub struct RecordAttr {}

impl Parse for RecordAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for RecordAttr {
    fn pretty_print(&self, _printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {}
}

/// A struct accepted by `#[record]`, with named fields and no generics.
pub struct RecordItem {
    pub item: ItemStruct,
}

impl Parse for RecordItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemStruct = input.parse()?;
        if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
            return Err(syn::Error::new_spanned(
                &item.generics,
                "records cannot have generic parameters",
            ));
        }
        let Fields::Named(fields) = &item.fields else {
            return Err(syn::Error::new_spanned(
                &item.fields,
                "records must have named fields",
            ));
        };
        for field in &fields.named {
            if let Some(attr) = field
                .attrs
                .iter()
                .find(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr"))
            {
                return Err(syn::Error::new_spanned(
                    attr,
                    "record fields cannot be configured conditionally",
                ));
            }
            let ident = field.ident.as_ref().expect("named fields have identifiers");
            if RESERVED_FIELDS.contains(&ident.unraw().to_string().as_str()) {
                return Err(syn::Error::new_spanned(
                    ident,
                    format!("`{ident}` is reserved and cannot name a record field"),
                ));
            }
        }
        Ok(Self { item })
    }
}

pub struct Record(RecordAttr, RecordItem);

impl Record {
    #[must_use]
    pub fn new(attr: RecordAttr, item: RecordItem) -> Self {
        Self(attr, item)
    }

    /// Reads a record declaration from the attribute arguments and struct tokens.
    ///
    /// # Errors
    ///
    /// Fails if the attribute has arguments or the struct violates the record
    /// declaration rules, including field and generic parameter restrictions.
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Record {
    #[allow(clippy::too_many_lines)]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = &self.1.item;
        let name = &item.ident;
        // The surrogates are as visible as the record, since their trait
        // impls name it.
        let record_vis = &item.vis;
        let serde_crate = format!("{}::internal::serde", topcoat_runtime.path_string());
        let serde: syn::Path = syn::parse_str(&serde_crate).expect("resolved crate path is valid");
        // A local binding, since repetitions in `quote!` cannot name a constant.
        let runtime: syn::Path = syn::parse_quote!(#topcoat_runtime);

        let idents: Vec<_> = item.fields.iter().map(|field| &field.ident).collect();
        let vis: Vec<_> = item.fields.iter().map(|field| &field.vis).collect();
        // `Self` in a field type names the record, not the generated types.
        let tys: Vec<_> = item
            .fields
            .iter()
            .map(|field| replace_self(field.ty.to_token_stream(), name))
            .collect();
        let params: Vec<_> = (0..tys.len())
            .map(|index| format_ident!("__T{index}"))
            .collect();

        let owned_tys: Vec<_> = tys
            .iter()
            .map(|ty| quote_spanned! {ty.span()=> <#ty as #runtime::Surrogated>::Surrogate })
            .collect();
        let borrowed_tys: Vec<_> = tys
            .iter()
            .map(|ty| {
                quote_spanned! {ty.span()=> <&'__a #ty as #runtime::Surrogated>::Surrogate }
            })
            .collect();

        quote! {
            #item

            const _: () = {
                #[doc(hidden)]
                #[allow(dead_code, non_camel_case_types)]
                #record_vis struct __TopcoatRecord {
                    #(#vis #idents: #owned_tys,)*
                }

                #[doc(hidden)]
                #[allow(dead_code, non_camel_case_types)]
                #record_vis struct __TopcoatRecordRef<'__a> {
                    #(#vis #idents: #borrowed_tys,)*
                    __topcoat_real: &'__a #name,
                }

                #[derive(#serde::Serialize, #serde::Deserialize)]
                #[serde(crate = #serde_crate, deny_unknown_fields)]
                #[allow(non_camel_case_types)]
                struct __TopcoatRecordFields<#(#params),*> {
                    #(#idents: #params,)*
                }

                impl #runtime::Surrogated for #name {
                    type Surrogate = __TopcoatRecord;

                    fn into_surrogate(self) -> Self::Surrogate {
                        __TopcoatRecord {
                            #(#idents: #runtime::Surrogated::into_surrogate(self.#idents),)*
                        }
                    }
                }

                impl #runtime::Surrogate for __TopcoatRecord {
                    type Real = #name;

                    fn into_real(self) -> Self::Real {
                        #name {
                            #(#idents: #runtime::Surrogate::into_real(self.#idents),)*
                        }
                    }
                }

                impl<'__a> #runtime::Surrogated for &'__a #name {
                    type Surrogate = __TopcoatRecordRef<'__a>;

                    fn into_surrogate(self) -> Self::Surrogate {
                        __TopcoatRecordRef {
                            #(#idents: #runtime::Surrogated::into_surrogate(&self.#idents),)*
                            __topcoat_real: self,
                        }
                    }
                }

                impl<'__a> #runtime::Surrogate for __TopcoatRecordRef<'__a> {
                    type Real = &'__a #name;

                    fn into_real(self) -> Self::Real {
                        self.__topcoat_real
                    }
                }

                // The higher-ranked bounds keep cloning conditional: a
                // record without `Clone` still compiles, it only cannot be
                // cloned.

                impl<'__a> __TopcoatRecordRef<'__a> {
                    /// Returns an owned copy of the borrowed record.
                    #[must_use]
                    #[allow(clippy::should_implement_trait)]
                    pub fn clone(&self) -> __TopcoatRecord
                    where
                        for<'__r> #name: ::core::clone::Clone,
                    {
                        #runtime::Surrogated::into_surrogate(
                            ::core::clone::Clone::clone(self.__topcoat_real),
                        )
                    }
                }

                impl ::core::clone::Clone for __TopcoatRecord
                where
                    #(for<'__r> #owned_tys: ::core::clone::Clone,)*
                {
                    fn clone(&self) -> Self {
                        Self {
                            #(#idents: ::core::clone::Clone::clone(&self.#idents),)*
                        }
                    }
                }

                // The serde impls are unconditional, since bounds on the
                // fields would be cyclic for recursive records.

                impl #serde::Serialize for __TopcoatRecord {
                    fn serialize<__S>(
                        &self,
                        serializer: __S,
                    ) -> ::core::result::Result<__S::Ok, __S::Error>
                    where
                        __S: #serde::Serializer,
                    {
                        #runtime::internal::serialize_record(
                            serializer,
                            &__TopcoatRecordFields { #(#idents: &self.#idents,)* },
                        )
                    }
                }

                impl<'__a> #serde::Serialize for __TopcoatRecordRef<'__a> {
                    fn serialize<__S>(
                        &self,
                        serializer: __S,
                    ) -> ::core::result::Result<__S::Ok, __S::Error>
                    where
                        __S: #serde::Serializer,
                    {
                        #runtime::internal::serialize_record(
                            serializer,
                            &__TopcoatRecordFields { #(#idents: &self.#idents,)* },
                        )
                    }
                }

                impl<'__de> #serde::Deserialize<'__de> for __TopcoatRecord {
                    fn deserialize<__D>(deserializer: __D) -> ::core::result::Result<Self, __D::Error>
                    where
                        __D: #serde::Deserializer<'__de>,
                    {
                        let fields: __TopcoatRecordFields<#(#owned_tys),*> =
                            #runtime::internal::deserialize_record(deserializer)?;
                        ::core::result::Result::Ok(Self {
                            #(#idents: fields.#idents,)*
                        })
                    }
                }
            };
        }
        .to_tokens(tokens);
    }
}

/// Substitutes the record's name for `Self` throughout a token stream.
fn replace_self(tokens: TokenStream, name: &Ident) -> TokenStream {
    tokens
        .into_iter()
        .map(|token| match token {
            TokenTree::Ident(ident) if ident == "Self" => {
                let mut name = name.clone();
                name.set_span(ident.span());
                TokenTree::Ident(name)
            }
            TokenTree::Group(group) => {
                let mut replaced =
                    Group::new(group.delimiter(), replace_self(group.stream(), name));
                replaced.set_span(group.span());
                TokenTree::Group(replaced)
            }
            other => other,
        })
        .collect()
}
