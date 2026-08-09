use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Expr, LitStr, Token, parse::Parse, punctuated::Punctuated};

enum ValidateAttr {
    String,
    Number,
    Bool,
    Required,
    Email,
    MinLength(Expr),
    MaxLength(Expr),
    Min(Expr),
    Max(Expr),
    OneOf(LitStr),
    OrDefault(Expr),
    Custom(Expr),
    ServerOnly,
    ClientOnly,
    Both,
}

impl Parse for ValidateAttr {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let name: syn::Ident = input.parse()?;

        if input.parse::<Token![=]>().is_ok() {
            let value: Expr = input.parse()?;
            match name.to_string().as_str() {
                "min_length" => Ok(ValidateAttr::MinLength(value)),
                "max_length" => Ok(ValidateAttr::MaxLength(value)),
                "min" => Ok(ValidateAttr::Min(value)),
                "max" => Ok(ValidateAttr::Max(value)),
                "one_of" => {
                    let lit = match value {
                        Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(s),
                            ..
                        }) => s,
                        _ => {
                            return Err(syn::Error::new_spanned(
                                value,
                                "one_of expects a comma-separated string literal",
                            ));
                        }
                    };
                    Ok(ValidateAttr::OneOf(lit))
                }
                "or_default" => Ok(ValidateAttr::OrDefault(value)),
                "custom" => Ok(ValidateAttr::Custom(value)),
                other => Err(syn::Error::new(
                    name.span(),
                    format!("unknown validator attribute '{other}'"),
                )),
            }
        } else {
            match name.to_string().as_str() {
                "string" => Ok(ValidateAttr::String),
                "number" => Ok(ValidateAttr::Number),
                "bool" => Ok(ValidateAttr::Bool),
                "required" => Ok(ValidateAttr::Required),
                "email" => Ok(ValidateAttr::Email),
                "server_only" => Ok(ValidateAttr::ServerOnly),
                "client_only" => Ok(ValidateAttr::ClientOnly),
                "both" => Ok(ValidateAttr::Both),
                other => Err(syn::Error::new(
                    name.span(),
                    format!("unknown validator attribute '{other}'"),
                )),
            }
        }
    }
}

pub fn derive(input: &DeriveInput) -> TokenStream {
    let ident = &input.ident;

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    input,
                    "FormSchema can only be derived for structs with named fields",
                )
                .into_compile_error();
            }
        },
        _ => {
            return syn::Error::new_spanned(input, "FormSchema can only be derived for structs")
                .into_compile_error();
        }
    };

    let field_calls = match fields
        .iter()
        .map(|field| {
            let field_ident = field.ident.as_ref().expect("named field has an ident");
            let attrs = parse_validate_attrs(field)?;
            let chain = field_chain(attrs);
            Ok(quote! { .field(stringify!(#field_ident), #chain) })
        })
        .collect::<Result<Vec<_>, syn::Error>>()
    {
        Ok(calls) => calls,
        Err(err) => return err.into_compile_error(),
    };

    quote! {
        impl ::topcoat::validation::FormSchema for #ident {
            fn schema() -> ::topcoat::validation::Schema {
                ::topcoat::validation::Schema::new()
                    #(#field_calls)*
            }
        }
    }
}

fn parse_validate_attrs(field: &syn::Field) -> Result<Vec<ValidateAttr>, syn::Error> {
    let mut attrs = Vec::new();
    for attr in &field.attrs {
        if attr.path().is_ident("validate") {
            let list: Punctuated<ValidateAttr, Token![,]> =
                attr.parse_args_with(Punctuated::parse_terminated)?;
            attrs.extend(list);
        }
    }
    Ok(attrs)
}

fn field_chain(attrs: Vec<ValidateAttr>) -> TokenStream {
    let mut tokens = quote! { ::topcoat::validation::Field::new() };
    for attr in attrs {
        let call = match attr {
            ValidateAttr::String => quote! { .string() },
            ValidateAttr::Number => quote! { .number() },
            ValidateAttr::Bool => quote! { .bool() },
            ValidateAttr::Required => quote! { .required() },
            ValidateAttr::Email => quote! { .email() },
            ValidateAttr::MinLength(value) => quote! { .min_length(#value) },
            ValidateAttr::MaxLength(value) => quote! { .max_length(#value) },
            ValidateAttr::Min(value) => quote! { .min(#value) },
            ValidateAttr::Max(value) => quote! { .max(#value) },
            ValidateAttr::OneOf(lit) => {
                let value = lit.value();
                let options: Vec<_> = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                quote! { .one_of(&[#(#options),*]) }
            }
            ValidateAttr::OrDefault(value) => quote! { .or_default(#value) },
            ValidateAttr::Custom(path) => quote! { .custom(#path) },
            ValidateAttr::ServerOnly => quote! { .server_only() },
            ValidateAttr::ClientOnly => quote! { .client_only() },
            ValidateAttr::Both => quote! { .both() },
        };
        tokens = quote! { #tokens #call };
    }
    tokens
}
