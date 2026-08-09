use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, LitStr, Path};

use crate::form_schema::{field_chain, parse_validate_attrs};

pub fn derive(input: &DeriveInput) -> TokenStream {
    let ident = &input.ident;

    let form_view = match parse_form_view(input) {
        Ok(path) => path,
        Err(err) => return err.into_compile_error(),
    };

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    input,
                    "FormGroupSchema can only be derived for structs with named fields",
                )
                .into_compile_error();
            }
        },
        _ => {
            return syn::Error::new_spanned(
                input,
                "FormGroupSchema can only be derived for structs",
            )
            .into_compile_error();
        }
    };

    let mut control_inits = Vec::new();
    let mut value_setters = Vec::new();
    let mut assemble_lets = Vec::new();
    let mut assemble_names = Vec::new();

    for field in fields {
        let field_ident = field.ident.as_ref().expect("named field has an ident");
        let field_name = field_ident.to_string();
        let field_ty = &field.ty;

        let attrs = match parse_validate_attrs(field) {
            Ok(attrs) => attrs,
            Err(err) => return err.into_compile_error(),
        };
        let chain = field_chain(attrs);

        control_inits.push(quote! {
            group.control(
                #field_name,
                ::topcoat::form::FormControl::<#field_ty>::new(
                    default.#field_ident.clone(),
                    #chain,
                ),
            );
        });

        value_setters.push(quote! {
            if let Some(control) = group.get_mut::<#field_ty>(#field_name) {
                let raw = values
                    .field(#field_name)
                    .unwrap_or(::topcoat::form::Value::Missing);
                control.set_raw_value(env, raw).ok();
            }
        });

        assemble_lets.push(quote! {
            let #field_ident = match group.get::<#field_ty>(#field_name) {
                Some(control) if control.valid() => Some(control.value().clone()),
                Some(control) => {
                    for error in control.errors() {
                        errors.push(#field_name, error.clone());
                    }
                    None
                }
                None => {
                    errors.push(#field_name, "control is missing");
                    None
                }
            };
        });

        assemble_names.push(field_ident);
    }

    let assembler = quote! {
        group.assembler(|group| {
            let mut errors = ::topcoat::form::ValidationErrors::new();
            #(#assemble_lets)*
            if errors.is_empty() {
                Ok(Self {
                    #(#assemble_names: #assemble_names.unwrap(),)*
                })
            } else {
                Err(errors)
            }
        });
    };

    let control_accessors: Vec<TokenStream> = fields
        .iter()
        .map(|field| {
            let field_ident = field.ident.as_ref().expect("named field has an ident");
            let field_name = field_ident.to_string();
            let field_ty = &field.ty;
            let accessor_ident = format_ident!("{}_control", field_ident);
            let expect_message = LitStr::new(
                &format!("{field_name} control"),
                proc_macro2::Span::call_site(),
            );

            quote! {
                #[must_use]
                pub fn #accessor_ident(
                    group: &::topcoat::form::FormGroup<#ident>,
                ) -> &::topcoat::form::FormControl<#field_ty> {
                    let control: &::topcoat::form::FormControl<#field_ty> = group
                        .get(#field_name)
                        .expect(#expect_message);
                    control
                }
            }
        })
        .collect();

    let procedure = form_view.map(|form_view| {
        let update_ident = format_ident!(
            "__topcoat_form_update_{}",
            ident.to_string().to_snake_case()
        );

        quote! {
            impl #ident {
                #(#control_accessors)*

                /// Returns the generated server procedure that re-renders the
                /// form from a JSON-encoded map of current values.
                #[must_use]
                pub fn form_update_procedure() -> &'static ::topcoat::runtime::Procedure<(::std::string::String,), ::std::string::String> {
                    &#update_ident
                }

                /// Returns the JSON-encoded surrogate for
                /// [`form_update_procedure`](Self::form_update_procedure),
                /// ready to embed in a `data-form-update` attribute.
                #[must_use]
                pub fn form_update_procedure_json() -> ::std::string::String {
                    let surrogate = ::topcoat::runtime::Surrogated::into_surrogate(#update_ident);
                    ::serde_json::to_string(surrogate)
                        .expect("procedure surrogate serializes")
                }
            }

            #[::topcoat::runtime::procedure]
            async fn #update_ident(
                cx: &::topcoat::context::Cx,
                values_json: ::std::string::String,
            ) -> ::topcoat::Result<::std::string::String> {
                use ::topcoat::view::Component;

                let values: ::std::collections::HashMap<::std::string::String, ::std::string::String> =
                    ::serde_json::from_str(&values_json)
                        .map_err(|error| ::topcoat::Error::from(::std::io::Error::new(
                            ::std::io::ErrorKind::InvalidInput,
                            format!("invalid form values: {error}"),
                        )))?;
                // Live updates re-render the form even when some controls are
                // invalid, so the lossy variant keeps the group and its errors.
                let group = <#ident as ::topcoat::form::FormGroupSchema>::form_group_from_values_lossy(
                    ::topcoat::form::ValidationEnv::Server,
                    &values,
                );
                let props = #form_view::props_builder().group(&group).build();
                // The marker path is used as `Default::default()` so the same
                // construction works for both unit-struct and generic
                // (`PhantomData`) markers.
                #[allow(clippy::default_constructed_unit_structs)]
                let view = Component::render(
                    #form_view::default(),
                    cx,
                    props,
                )
                .await?;
                Ok(view.render(cx))
            }
        }
    });

    let accessors_only = if procedure.is_some() {
        quote! {}
    } else {
        quote! {
            impl #ident {
                #(#control_accessors)*
            }
        }
    };

    quote! {
        impl ::topcoat::form::FormGroupSchema for #ident {
            fn form_group() -> ::topcoat::form::FormGroup<Self>
            where
                Self: ::core::default::Default,
            {
                Self::form_group_with(&Self::default())
            }

            fn form_group_with(default: &Self) -> ::topcoat::form::FormGroup<Self> {
                let mut group = ::topcoat::form::FormGroup::new();
                #(#control_inits)*
                #assembler
                group
            }

            fn form_group_from_values(
                env: ::topcoat::form::ValidationEnv,
                values: &dyn ::topcoat::form::ValidationData,
            ) -> ::std::result::Result<::topcoat::form::FormGroup<Self>, ::topcoat::form::ValidationErrors> {
                let group = Self::form_group_from_values_lossy(env, values);
                if group.valid() {
                    Ok(group)
                } else {
                    Err(group.errors())
                }
            }

            fn form_group_from_values_lossy(
                env: ::topcoat::form::ValidationEnv,
                values: &dyn ::topcoat::form::ValidationData,
            ) -> ::topcoat::form::FormGroup<Self> {
                let mut group = Self::form_group();
                #(#value_setters)*
                group
            }
        }

        #procedure

        #accessors_only
    }
}

fn parse_form_view(input: &DeriveInput) -> Result<Option<Path>, syn::Error> {
    for attr in &input.attrs {
        if attr.path().is_ident("form_view") {
            match &attr.meta {
                syn::Meta::List(list) => return list.parse_args::<Path>().map(Some),
                syn::Meta::NameValue(nv) => {
                    let path = match &nv.value {
                        syn::Expr::Path(expr) => expr.path.clone(),
                        _ => {
                            return Err(syn::Error::new_spanned(
                                &nv.value,
                                "form_view expects a path to a component",
                            ));
                        }
                    };
                    return Ok(Some(path));
                }
                syn::Meta::Path(_) => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        "form_view expects a component path: #[form_view(component)] or #[form_view = component]",
                    ));
                }
            }
        }
    }
    Ok(None)
}
