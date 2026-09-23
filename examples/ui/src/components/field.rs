use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

use super::label::label;

/// The layout of a [`field`].
///
/// The default is `FieldOrientation::Vertical`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FieldOrientation {
    /// Stacks the label, the control, and the text below them.
    #[default]
    Vertical,
    /// Places the control next to its label or [`field_content`].
    Horizontal,
    /// Stacks vertically, and switches to a row when the surrounding
    /// [`field_group`] is wide enough.
    Responsive,
}

impl FieldOrientation {
    /// The Tailwind classes for this orientation.
    fn classes(self) -> StaticClass {
        match self {
            Self::Vertical => class!("flex-col gap-2"),
            Self::Horizontal => {
                class!("flex-row items-center gap-3 [&>[data-slot=field-label]]:flex-1")
            }
            Self::Responsive => class!(
                "flex-col gap-2 @md/field-group:flex-row @md/field-group:items-start \
                 @md/field-group:gap-4 @md/field-group:[&>[data-slot=field-label]]:w-1/3 \
                 @md/field-group:[&>[data-slot=field-label]]:shrink-0",
            ),
        }
    }
}

/// The text size of a [`field_legend`].
///
/// The default is `FieldLegendVariant::Legend`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FieldLegendVariant {
    /// A heading for a section of a form.
    #[default]
    Legend,
    /// A smaller heading that matches a field label.
    Label,
}

impl FieldLegendVariant {
    /// The Tailwind classes for this variant.
    fn classes(self) -> StaticClass {
        match self {
            Self::Legend => class!("text-base font-semibold"),
            Self::Label => class!("text-sm font-medium"),
        }
    }
}

/// A group of related controls, rendered as a `<fieldset>` and named by a
/// [`field_legend`].
///
/// The `attrs` are forwarded to the `<fieldset>`. Pass `disabled` to disable
/// all of its controls. A `class` among the `attrs` is appended to the
/// component's classes. The same holds for the other field components.
#[component]
pub async fn field_set(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <fieldset
            class=(class!("flex min-w-0 flex-col gap-5", attrs.remove("class")))
            (attrs)
        >
            (child)
        </fieldset>
    })
}

/// The heading of a [`field_set`], rendered as a `<legend>`.
///
/// Place it first in the set. `variant` sets the text size and defaults to
/// `Legend`.
#[component]
pub async fn field_legend(
    #[default] variant: FieldLegendVariant,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <legend
            class=(class!("mb-3", variant.classes(), attrs.remove("class")))
            (attrs)
        >
            (child)
        </legend>
    })
}

/// A vertical stack of fields.
///
/// It is a container query container, which a [`field`] with the
/// `Responsive` orientation uses to pick its layout.
#[component]
pub async fn field_group(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "@container/field-group flex flex-col gap-5",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}

/// A control together with its label, description, and error message.
///
/// Put any control inside it, such as an input, a select, or a checkbox. Set
/// `for` on the [`field_label`] to the control's `id`, and point the
/// control's `aria-describedby` at the ids of the description and the error.
/// When the control has `aria-invalid="true"`, the label turns the
/// destructive color too. A `data-invalid="true"` attribute on the field has
/// the same effect. The field does not validate anything or decide when to
/// show errors. That is up to you.
///
/// `orientation` sets the layout and defaults to `Vertical`. The `attrs` are
/// forwarded to the wrapping `<div>`, which has `role="group"`.
#[component]
pub async fn field(
    #[default] orientation: FieldOrientation,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            role="group"
            data-slot="field"
            class=(class!(
                "group/field flex min-w-0",
                orientation.classes(),
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}

/// A column that groups a field's label, control, or description, for
/// example next to a control in a horizontal [`field`].
#[component]
pub async fn field_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!("flex min-w-0 flex-1 flex-col gap-1.5", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// A [`label`] that follows the disabled and invalid states of its [`field`].
///
/// Set `for` to the control's `id` to connect the label to it.
#[component]
pub async fn field_label(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        label(
            attrs: attributes! {
                data-slot="field-label"
                class=(class!(
                    "leading-snug group-has-[:disabled]/field:opacity-50 \
                     group-has-[[aria-invalid=true]]/field:text-destructive \
                     group-data-[invalid=true]/field:text-destructive",
                    attrs.remove("class"),
                ))
                (attrs)
            },
            (child)
        )
    })
}

/// Text styled like a label that does not label a control.
///
/// To label a control, use [`field_label`] instead.
#[component]
pub async fn field_title(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!("text-sm leading-snug font-medium", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// Muted text that describes a control, rendered as a `<p>`.
///
/// Give it an `id` and add that id to the control's `aria-describedby`.
#[component]
pub async fn field_description(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <p
            class=(class!(
                "text-sm leading-relaxed text-muted-foreground [&_a]:underline [&_a]:underline-offset-4",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </p>
    })
}

/// A horizontal line between sections of a form, with optional text in the
/// middle.
///
/// Child nodes become the text. Without children, the line is unbroken.
#[component]
pub async fn field_separator(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "flex items-center gap-3 text-xs text-muted-foreground has-[>span:empty]:gap-0 before:h-px before:flex-1 before:bg-border after:h-px after:flex-1 after:bg-border",
                attrs.remove("class"),
            ))
            (attrs)
        >
            <span class="empty:hidden">(child)</span>
        </div>
    })
}

/// An error message for a field, announced by screen readers when it
/// appears.
///
/// Render it only when there is an error. Give it an `id` and add that id to
/// the control's `aria-describedby`, and set `aria-invalid="true"` on the
/// control. Child nodes become the content, such as one message or a list of
/// messages.
#[component]
pub async fn field_error(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            role="alert"
            class=(class!("text-sm text-destructive", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}
