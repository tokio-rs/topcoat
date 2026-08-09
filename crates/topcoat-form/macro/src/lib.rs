use proc_macro::TokenStream;
use quote::quote;
use syn::{Expr, LitStr, Path, Token, parse::Parse, parse_macro_input};

mod client_form;
mod form_group_schema;
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

struct FormGroupHandlers {
    form_type: Path,
    target: LitStr,
}

impl Parse for FormGroupHandlers {
    fn parse(input: syn::parse::ParseStream<'_>) -> syn::Result<Self> {
        let form_type: Path = input.parse()?;
        let target = if input.parse::<Token![,]>().is_ok() && !input.is_empty() {
            input.parse::<LitStr>()?
        } else {
            use heck::ToSnakeCase;
            let ident = form_type
                .segments
                .last()
                .expect("form type path is empty")
                .ident
                .to_string();
            LitStr::new(
                &format!("topcoat-form-{}", ident.to_snake_case()),
                proc_macro2::Span::call_site(),
            )
        };
        Ok(Self { form_type, target })
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
/// [`FormSchema`](::topcoat::form::FormSchema) and derive
/// [`ClientForm`](::topcoat::form::ClientForm), which generates the
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

/// Attaches live-update handlers to a [`FormGroup`] form.
///
/// The first argument is the form type. The type must derive
/// [`ValidForm`](::topcoat::form::ValidForm) and be annotated with
/// `#[form_view(path)]`, which generates the server-side update procedure.
/// An optional second argument is the `data-form-target` selector; when
/// omitted, a stable target is derived from the form type name.
///
/// The generated attributes wire `@input`, `@change`, and `@focusout` so that
/// every keystroke or blur sends the current form values to the server,
/// re-renders the form fragment, and swaps it into the DOM.
#[proc_macro]
pub fn form_group_handlers(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as FormGroupHandlers);
    let form_type = &args.form_type;
    let target = args.target.value();

    let update_js = |state_attr: &str| {
        format!(
            r#"async (e) => {{
    const input = e.target;
    if (!input || !input.hasAttribute || !input.hasAttribute('data-control-name')) return;
    const form = input.closest('[data-form-target="{target}"]');
    if (!form) return;
    input.setAttribute('data-control-{state_attr}', 'true');
    input.setAttribute(
        'data-control-value',
        input.type === 'checkbox' ? (input.checked ? 'true' : 'false') : input.value,
    );
    const values = {{}};
    for (const el of form.querySelectorAll('[data-control-name]')) {{
        const name = el.getAttribute('data-control-name');
        values[name] = el.getAttribute('data-control-value') || '';
    }}
    const procedure = cx.hydrate(JSON.parse(form.getAttribute('data-form-update')));
    const html = await procedure.call(JSON.stringify(values));
    cx.swapHtml('[data-form-target="{target}"]', html.toString(), {{ mode: 'outer' }});
}}"#,
        )
    };

    let input_js = update_js("dirty");
    let change_js = update_js("dirty");
    let focusout_js = update_js("touched");

    quote! {
        ::topcoat::view::attributes! {
            ("data-form-target") = #target
            ("data-form-update") = (#form_type::form_update_procedure_json())
            @input = #input_js
            @change = #change_js
            @focusout = #focusout_js
        }
    }
    .into()
}

/// Derives [`ValidationData`](::topcoat::form::ValidationData) for a
/// struct with named fields.
///
/// Supported field types are `String`, numeric primitives, `bool`, and
/// `Option<T>` where `T` is one of those types.
#[proc_macro_derive(ValidationData)]
pub fn derive_validation_data(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    validation_data::derive(&input).into()
}

/// Derives [`FormSchema`](::topcoat::form::FormSchema) from
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
/// The type must implement [`FormSchema`](::topcoat::form::FormSchema).
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
/// [`FormSchema`](derive@FormSchema), [`ClientForm`](derive@ClientForm), and
/// [`FormGroupSchema`](derive@FormGroupSchema).
///
/// Add `#[form_view(path::to_component)]` to the struct to enable live,
/// server-authoritative form updates. The derive then generates an update
/// procedure that re-renders `path::to_component` on every input/blur and
/// returns the HTML fragment for the client to swap into the DOM. The
/// component must accept a `group: &FormGroup<Self>` prop. Use
/// [`form_group_handlers!`](macro@form_group_handlers) on the `<form>` element
/// to wire the client-side events.
#[proc_macro_derive(ValidForm, attributes(validate, form_view))]
pub fn derive_valid_form(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    let validation_data = validation_data::derive(&input);
    let form_schema = form_schema::derive(&input);
    let client_form = client_form::derive(&input);
    let form_group_schema = form_group_schema::derive(&input);
    quote! {
        #validation_data
        #form_schema
        #client_form
        #form_group_schema
    }
    .into()
}
