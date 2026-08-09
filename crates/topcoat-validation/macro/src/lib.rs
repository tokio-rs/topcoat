use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, LitStr, Path, Token, parse::Parse, parse_macro_input};

mod client_form;
mod form_schema;
mod shared;
mod validation_data;

struct FormValidationHandlers {
    form_type: Path,
    errors: Expr,
    form_id: Option<LitStr>,
}

impl Parse for FormValidationHandlers {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let form_type = input.parse()?;
        input.parse::<Token![,]>()?;
        let errors = input.parse()?;
        let form_id = if input.parse::<Token![,]>().is_ok() && !input.is_empty() {
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self {
            form_type,
            errors,
            form_id,
        })
    }
}

fn validator_path_for(form_type: &Path) -> Path {
    use heck::ToSnakeCase;
    use quote::format_ident;

    let mut path = form_type.clone();
    let last = path.segments.last_mut().expect("form type path is empty");
    last.ident = format_ident!(
        "__topcoat_validate_{}",
        last.ident.to_string().to_snake_case()
    );
    path
}

/// Attaches `@input`, `@change`, and `@submit` handlers to a form so it
/// validates on the client before submitting.
///
/// The first argument is the form type. The type must implement
/// [`FormSchema`](::topcoat::validation::FormSchema) and derive
/// [`ClientForm`](::topcoat::validation::ClientForm), which generates the
/// client-side validator procedure. The second argument is a `Signal<String>`
/// that receives the error text. An optional third argument is the `id` of the
/// form; when omitted, a stable id is derived from the form type name.
#[proc_macro]
pub fn form_validation_handlers(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as FormValidationHandlers);
    let validator = validator_path_for(&args.form_type);
    let errors = &args.errors;

    let form_id = args.form_id.map(|l| l.value()).unwrap_or_else(|| {
        use heck::ToSnakeCase;
        let ident = args
            .form_type
            .segments
            .last()
            .expect("form type path is empty")
            .ident
            .to_string();
        format!("topcoat-form-{}", ident.to_snake_case())
    });

    let input_change_js = format!(
        "cx.hydrate(new URLSearchParams(new FormData(document.getElementById(\"{}\"))).toString())",
        form_id
    );
    let submit_check_js = format!(
        "if (${{errors}}.dehydrate().length === 0) {{ document.getElementById(\"{}\").submit(); }}",
        form_id
    );

    quote! {
        ::topcoat::view::attributes! {
            id = #form_id
            @input=$(async |e: ::topcoat::runtime::Event| {
                let errors = #validator(raw!(
                    #input_change_js,
                    String::new()
                )).await;
                #errors.set(errors);
            })
            @change=$(async |e: ::topcoat::runtime::Event| {
                let errors = #validator(raw!(
                    #input_change_js,
                    String::new()
                )).await;
                #errors.set(errors);
            })
            @submit=$(async |e: ::topcoat::runtime::Event| {
                e.prevent_default();
                let errors = #validator(raw!(
                    #input_change_js,
                    String::new()
                )).await;
                #errors.set(errors);
                raw!(
                    #submit_check_js,
                    ()
                );
            })
        }
    }
    .into()
}

/// Derives [`ValidationData`](::topcoat::validation::ValidationData) for a
/// struct with named fields.
///
/// Supported field types are `String`, numeric primitives, `bool`, and
/// `Option<T>` where `T` is one of those types.
#[proc_macro_derive(ValidationData)]
pub fn derive_validation_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    validation_data::derive(&input).into()
}

/// Derives [`FormSchema`](::topcoat::validation::FormSchema) from
/// `#[validate(...)]` field attributes.
///
/// Supported validators: `string`, `number`, `bool`, `required`, `email`,
/// `min_length`, `max_length`, `min`, `max`, `one_of`, `or_default`, `custom`,
/// `server_only`, `client_only`, `both`.
#[proc_macro_derive(FormSchema, attributes(validate))]
pub fn derive_form_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    form_schema::derive(&input).into()
}

/// Derives the client-side validator for a form type.
///
/// The type must implement [`FormSchema`](::topcoat::validation::FormSchema).
/// This derive generates a `#[procedure]` that accepts a URL-encoded form
/// string and returns an error message (empty when valid).
#[proc_macro_derive(ClientForm)]
pub fn derive_client_form(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    client_form::derive(&input).into()
}

/// Derives everything needed for a validated form.
///
/// Expands to [`ValidationData`](derive@ValidationData),
/// [`FormSchema`](derive@FormSchema), and [`ClientForm`](derive@ClientForm).
#[proc_macro_derive(ValidForm, attributes(validate))]
pub fn derive_valid_form(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let validation_data = validation_data::derive(&input);
    let form_schema = form_schema::derive(&input);
    let client_form = client_form::derive(&input);
    quote! {
        #validation_data
        #form_schema
        #client_form
    }
    .into()
}
