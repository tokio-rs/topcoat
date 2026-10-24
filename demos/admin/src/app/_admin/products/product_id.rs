use topcoat::{
    Result,
    context::{Cx, memoize, try_request_context},
    l10n::Translation,
    router::{
        Body, Method, Next, StatusCode,
        content::Form,
        error::{RouterErrorExt, SeeOther, rewrite, see_other},
        href, layer, module_param, page, path_param,
        response::Response,
        route,
    },
    runtime::link,
    view::{Attributes, View, attributes, class, component, element_id, view},
};

use super::{stock_badge, t};
use crate::{
    app::{_admin::Crumb, _common::money_input},
    components::{
        button::{ButtonSize, ButtonVariant, button, button_variants},
        card::{card, card_content, card_description, card_footer, card_header, card_title},
        field::{field, field_error, field_group, field_label, field_set},
        input::input,
    },
    features::{
        auth::require_user,
        catalog::{Product, ProductError, ProductErrors, ProductForm},
    },
};

module_param!(pub product_id: u64, error = not_found);

struct Rejected {
    form: ProductForm,
    errors: ProductErrors,
}

#[memoize(as_ref)]
async fn product(cx: &Cx) -> Result<Product> {
    let id = *path_param::<ProductId>(cx)?;
    Ok(Product::find(cx, id).await?.ok_or_not_found()?)
}

#[layer]
async fn crumb(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let name = product(cx).await?.name.clone();
    let cx = cx.with(Crumb(name));
    next.run(&cx, body).await
}

#[page]
pub async fn page(cx: &Cx) -> Result<impl View> {
    let user = require_user(cx).await?;
    let product = product(cx).await?;
    let can_edit = user.role.can_edit();

    let rejected = try_request_context::<Rejected>(cx);
    let form = rejected.map_or_else(
        || ProductForm {
            price: money_input(product.price).resolve(cx).into_owned(),
            ..ProductForm::of(product)
        },
        |rejected| rejected.form.clone(),
    );
    let errors = rejected.map(|rejected| rejected.errors).unwrap_or_default();
    let form_id = element_id(cx);

    Ok(view! {
        if rejected.is_some() {
            (StatusCode::UNPROCESSABLE_ENTITY)
        }

        <header class="flex flex-col gap-1">
            <div class="flex items-center gap-3">
                <h1 class="text-2xl font-semibold tracking-tight">(&product.name)</h1>
                stock_badge(level: product.stock_level())
            </div>
            <p class="font-mono text-sm text-muted-foreground">(&product.sku)</p>
        </header>

        <div class="grid grid-cols-1 gap-6 lg:grid-cols-3 lg:items-start">
            card(
                attrs: attributes! { class="lg:col-span-2" },
                card_header(
                    card_title((t!(details)))
                    card_description((t!(details.description)))
                )
                card_content(
                    <form
                        id=(&form_id)
                        method="post"
                        action=(href!(save, ProductId(product.id)))
                    >
                        field_set(
                            attrs: attributes! { disabled=((!can_edit).then_some("")) },
                            field_group(
                                product_field(
                                    label: t!(field_name),
                                    name: "name",
                                    value: &form.name,
                                    error: errors.name.map(error_message)
                                )
                                product_field(
                                    label: t!(field_sku),
                                    name: "sku",
                                    value: &form.sku,
                                    error: errors.sku.map(error_message),
                                    attrs: attributes! { class="font-mono uppercase" }
                                )
                                <div class="grid grid-cols-1 gap-6 sm:grid-cols-2">
                                    product_field(
                                        label: t!(field_price),
                                        name: "price",
                                        value: &form.price,
                                        error: errors.price.map(error_message),
                                        attrs: attributes! { inputmode="decimal" }
                                    )
                                    product_field(
                                        label: t!(field_stock),
                                        name: "stock",
                                        value: &form.stock,
                                        error: errors.stock.map(error_message),
                                        attrs: attributes! { inputmode="numeric" }
                                    )
                                </div>
                            )
                        )
                    </form>
                )
            )

            card(
                card_header(
                    if can_edit {
                        card_title((t!(save)))
                        card_description((t!(save.description)))
                    } else {
                        card_title((t!(view_only)))
                        card_description((t!(view_only.description)))
                    }
                )
                if can_edit {
                    card_footer(
                        attrs: attributes! { class="flex-col gap-3" },
                        button(
                            attrs: attributes! { type="submit" form=(&form_id) class="w-full" },
                            (t!(save))
                        )
                        link(
                            href: href!(super::page),
                            attrs: attributes! {
                                class=(class!(
                                    button_variants(ButtonVariant::Outline, ButtonSize::Md),
                                    "w-full",
                                ))
                            },
                            (t!(cancel))
                        )
                    )
                }
            )
        </div>
    })
}

/// Renders a labeled field with an optional error. Extra `attrs` go on the input.
#[component]
async fn product_field(
    cx: &Cx,
    label: Translation<'_>,
    name: &str,
    value: &str,
    error: Option<Translation<'_>>,
    #[default] attrs: Attributes,
) -> Result<impl View> {
    let input_id = element_id(cx);
    let error_id = element_id(cx);
    let invalid = error.is_some();

    Ok(view! {
        field(
            field_label(attrs: attributes! { for=(&input_id) }, (label))
            input(
                attrs: attributes! {
                    id=(&input_id)
                    name=(name)
                    value=(value)
                    autocomplete="off"
                    aria-invalid=(invalid.then_some("true"))
                    aria-describedby=(invalid.then_some(&error_id))
                    (attrs)
                }
            )
            if let Some(error) = error {
                field_error(attrs: attributes! { id=(&error_id) }, (error))
            }
        )
    })
}

fn error_message(error: ProductError) -> Translation<'static> {
    match error {
        ProductError::NameMissing => t!(error_name_missing),
        ProductError::NameTooLong { max } => t!(error_name_too_long, max),
        ProductError::SkuMissing => t!(error_sku_missing),
        ProductError::SkuInvalid => t!(error_sku_invalid),
        ProductError::SkuTooLong { max } => t!(error_sku_too_long, max),
        ProductError::SkuTaken => t!(error_sku_taken),
        ProductError::PriceMissing => t!(error_price_missing),
        ProductError::PriceNotPositive => t!(error_price_not_positive),
        ProductError::PriceInvalid => t!(error_price_invalid),
        ProductError::StockMissing => t!(error_stock_missing),
        ProductError::StockInvalid => t!(error_stock_invalid),
    }
}

#[route(POST)]
pub async fn save(cx: &Cx, Form(form): Form<ProductForm>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    user.role.can_edit().then_some(()).ok_or_forbidden()?;

    let id = *path_param::<ProductId>(cx)?;
    let mut product = Product::find(cx, id).await?.ok_or_not_found()?;
    if let Err(errors) = product.edit(cx, &form).await? {
        // Rewrite the POST to keep the submitted values and errors in the request context.
        return Err(
            rewrite(href!(page, ProductId(id)).resolve(cx), Body::empty())
                .method(Method::GET)
                .with(Rejected { form, errors })
                .into(),
        );
    }

    Ok(see_other(href!(super::page).resolve(cx)))
}
