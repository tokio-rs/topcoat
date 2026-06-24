use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Fields, GenericParam, Generics, Ident, Type, Visibility,
    ext::IdentExt, parse_quote,
};

/// A parsed `#[derive(Props)]` struct. Expands into a typestate builder where
/// every field without `#[default]` must be set before `build()` becomes
/// available, plus implementations of [`topcoat::view::Props`] and an inherent
/// `builder()` function on the props struct.
///
/// Each required field is tracked by a generic argument of the builder. It
/// starts out as a generated marker type named after the field and flips to
/// [`topcoat::view::Set`] when the field's setter is called. `build()` bounds
/// every marker by [`topcoat::view::IsSet`], so forgetting a field produces a
/// "missing required property" error naming the field.
///
/// [`topcoat::view::IsSet`]: trait.IsSet.html
/// [`topcoat::view::Props`]: trait.Props.html
/// [`topcoat::view::Set`]: struct.Set.html
pub struct Props {
    vis: Visibility,
    ident: Ident,
    generics: Generics,
    fields: Vec<PropsField>,
}

struct PropsField {
    ident: Ident,
    ty: Type,
    /// The field's doc comments (`#[doc = ...]` attributes), forwarded onto the
    /// generated setter method.
    docs: Vec<Attribute>,
    /// `#[into]`: the setter accepts `impl Into<T>`.
    into: bool,
    /// The typestate parameter tracking whether this field has been set.
    /// `None` for `#[default]` fields, which never need to be set.
    state: Option<Ident>,
}

impl Props {
    /// Parses a `#[derive(Props)]` input from a token stream.
    ///
    /// # Errors
    ///
    /// Returns an error if the token stream fails to parse as a [`DeriveInput`],
    /// or if the parsed input is not a struct with named fields.
    pub fn parse(item: TokenStream) -> syn::Result<Self> {
        Self::new(syn::parse2(item)?)
    }

    /// Builds a [`Props`] from a parsed derive input.
    ///
    /// # Errors
    ///
    /// Returns an error if `input` is not a struct with named fields, or if a
    /// field is named `build` (which would clash with the generated `build()`
    /// method).
    ///
    /// # Panics
    ///
    /// Panics if a named field is missing its identifier. `syn` guarantees that
    /// named fields always have identifiers, so this never occurs in practice.
    pub fn new(input: DeriveInput) -> syn::Result<Self> {
        let Data::Struct(data) = input.data else {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "`Props` can only be derived for structs with named fields",
            ));
        };
        let Fields::Named(named) = data.fields else {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "`Props` can only be derived for structs with named fields",
            ));
        };

        let mut fields = Vec::new();
        for field in named.named {
            let ident = field.ident.expect("named field has an ident");
            if ident == "build" {
                return Err(syn::Error::new_spanned(
                    &ident,
                    "a field named `build` would clash with the generated `build()` method",
                ));
            }

            let mut default = false;
            let mut into = false;
            let mut docs = Vec::new();
            for attr in &field.attrs {
                if attr.path().is_ident("default") {
                    attr.meta.require_path_only()?;
                    default = true;
                } else if attr.path().is_ident("into") {
                    attr.meta.require_path_only()?;
                    into = true;
                } else if attr.path().is_ident("doc") {
                    docs.push(attr.clone());
                }
            }

            let state = (!default).then(|| format_ident!("__{}", ident.unraw()));
            fields.push(PropsField {
                ident,
                ty: field.ty,
                docs,
                into,
                state,
            });
        }

        Ok(Self {
            vis: input.vis,
            ident: input.ident,
            generics: input.generics,
            fields,
        })
    }
}

impl ToTokens for Props {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            vis,
            ident,
            generics,
            fields,
        } = self;
        let builder_ident = format_ident!("{ident}Builder");
        let members_mod = format_ident!("__{}", ident.unraw().to_string().to_snake_case());

        // Required fields paired with their typestate parameter.
        let required: Vec<(&Ident, &Ident)> = fields
            .iter()
            .filter_map(|f| f.state.as_ref().map(|state| (&f.ident, state)))
            .collect();
        let states: Vec<&Ident> = required.iter().map(|(_, state)| *state).collect();
        let field_idents: Vec<&Ident> = fields.iter().map(|f| &f.ident).collect();
        let field_tys: Vec<&Type> = fields.iter().map(|f| &f.ty).collect();

        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        // Generic arguments of the props struct, spliced in front of the
        // typestate marker arguments of the builder.
        let base_args: Vec<TokenStream> = generics
            .params
            .iter()
            .map(|param| match param {
                GenericParam::Lifetime(param) => param.lifetime.to_token_stream(),
                GenericParam::Type(param) => param.ident.to_token_stream(),
                GenericParam::Const(param) => param.ident.to_token_stream(),
            })
            .collect();

        let builder_ty = |state_args: &dyn Fn(&Ident, &Ident) -> TokenStream| {
            let args = base_args
                .iter()
                .cloned()
                .chain(
                    required
                        .iter()
                        .map(|(field, state)| state_args(field, state)),
                )
                .collect::<Vec<_>>();
            if args.is_empty() {
                quote!(#builder_ident)
            } else {
                quote!(#builder_ident<#(#args),*>)
            }
        };
        let unset_ty = builder_ty(&|field, _| quote!(#members_mod::#field));

        // Hidden marker types named after the required fields, so the
        // "missing required property" error names the field.
        let members = (!required.is_empty()).then(|| {
            let members = required.iter().map(|(field, _)| *field);
            quote! {
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                #vis mod #members_mod {
                    #(pub struct #members;)*
                }
            }
        });

        // Builder declaration generics: the struct's generics plus one marker
        // parameter per required field, defaulting to the unset marker.
        let mut decl_generics = generics.clone();
        for (field, state) in &required {
            decl_generics
                .params
                .push(parse_quote!(#state = #members_mod::#field));
        }

        // Setter impl generics: the struct's generics plus free marker
        // parameters, so setters are available in every builder state.
        let mut state_generics = generics.clone();
        for state in &states {
            state_generics.params.push(parse_quote!(#state));
        }
        let (state_impl_generics, state_ty_generics, _) = state_generics.split_for_impl();

        let phantom_field = (!states.is_empty()).then(|| {
            quote! { __props_state: ::core::marker::PhantomData<(#(#states,)*)>, }
        });
        let phantom_init = (!states.is_empty()).then(|| {
            quote! { __props_state: ::core::marker::PhantomData, }
        });

        let setters = fields.iter().map(|field| {
            let field_ident = &field.ident;
            let ty = &field.ty;
            // Carry the field's own doc comment onto the setter; fall back to a
            // generated summary when the field is undocumented.
            let doc = if field.docs.is_empty() {
                let generated = format!(" Sets the `{}` property.", field_ident.unraw());
                quote! { #[doc = #generated] }
            } else {
                let docs = &field.docs;
                quote! { #(#docs)* }
            };

            let (param_ty, value) = if field.into {
                (
                    quote!(impl ::core::convert::Into<#ty>),
                    quote!(::core::convert::Into::into(#field_ident)),
                )
            } else {
                (quote!(#ty), quote!(#field_ident))
            };

            // Required field: setting it flips its marker to `Set`, so the
            // builder has to be rebuilt under the new type.
            if let Some(state) = &field.state {
                let ret_ty = builder_ty(&|_, other| {
                    if other == state {
                        quote!(::topcoat::view::Set)
                    } else {
                        other.to_token_stream()
                    }
                });
                let other_idents = field_idents.iter().filter(|other| **other != field_ident);
                quote! {
                    #doc
                    #vis fn #field_ident(self, #field_ident: #param_ty) -> #ret_ty {
                        #builder_ident {
                            #field_ident: ::core::option::Option::Some(#value),
                            #(#other_idents: self.#other_idents,)*
                            #phantom_init
                        }
                    }
                }
            } else {
                // `#[default]` field: setting it does not change the typestate.
                quote! {
                    #doc
                    #vis fn #field_ident(mut self, #field_ident: #param_ty) -> Self {
                        self.#field_ident = ::core::option::Option::Some(#value);
                        self
                    }
                }
            }
        });

        let build_fields = fields.iter().map(|field| {
            let field_ident = &field.ident;
            if field.state.is_some() {
                quote! {
                    #field_ident: match self.#field_ident {
                        ::core::option::Option::Some(value) => value,
                        ::core::option::Option::None => ::core::unreachable!(),
                    },
                }
            } else {
                quote! {
                    #field_ident: self.#field_ident.unwrap_or_default(),
                }
            }
        });

        let builder_doc = format!(
            " Typestate builder for [`{ident}`], created by [`{ident}::builder()`]. \
             `build()` becomes available once every required property has been set.",
        );
        let builder_fn_doc = format!(" Returns a [`{builder_ident}`] with no properties set.");

        quote! {
            #members

            #[doc = #builder_doc]
            #[allow(non_camel_case_types)]
            #vis struct #builder_ident #decl_generics #where_clause {
                #(#field_idents: ::core::option::Option<#field_tys>,)*
                #phantom_field
            }

            #[automatically_derived]
            #[allow(non_camel_case_types, dead_code)]
            impl #state_impl_generics #builder_ident #state_ty_generics #where_clause {
                #(#setters)*

                /// Builds the props struct from the set properties.
                ///
                /// Only available once every required property has been set.
                /// `#[default]` properties that were not set are filled with
                /// `Default::default()`.
                #vis fn build(self) -> #ident #ty_generics
                where
                    #(#states: ::topcoat::view::IsSet,)*
                {
                    #ident {
                        #(#build_fields)*
                    }
                }
            }

            #[automatically_derived]
            #[allow(dead_code)]
            impl #impl_generics #ident #ty_generics #where_clause {
                #[doc = #builder_fn_doc]
                #vis fn builder() -> #unset_ty {
                    #builder_ident {
                        #(#field_idents: ::core::option::Option::None,)*
                        #phantom_init
                    }
                }
            }

            #[automatically_derived]
            impl #impl_generics ::topcoat::view::Props for #ident #ty_generics #where_clause {
                type Builder = #unset_ty;

                fn builder() -> Self::Builder {
                    Self::builder()
                }
            }
        }
        .to_tokens(tokens);
    }
}
