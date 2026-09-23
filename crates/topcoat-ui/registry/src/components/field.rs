use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

use super::label::label;

/// The layout of a [`field`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FieldOrientation {
    /// Stack the label, control, and supporting text.
    #[default]
    Vertical,
    /// Place a control beside its label or [`field_content`].
    Horizontal,
    /// Switch to a row when the enclosing [`field_group`] is wide enough.
    Responsive,
}

impl FieldOrientation {
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum FieldLegendVariant {
    /// A heading for a section of the form.
    #[default]
    Legend,
    /// A smaller heading matching a field label.
    Label,
}

impl FieldLegendVariant {
    fn classes(self) -> StaticClass {
        match self {
            Self::Legend => class!("text-base font-semibold"),
            Self::Label => class!("text-sm font-medium"),
        }
    }
}

/// A semantic group of related controls, named by a [`field_legend`].
///
/// Attributes are forwarded to the `<fieldset>`. Pass `disabled` to disable
/// its controls together. Classes are appended to the component's classes,
/// as with the other field components.
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

/// The accessible heading of a [`field_set`]. Place it first in the set.
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

/// A stack of fields, with a container for responsive field layouts.
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

/// A control and its label, description, and optional error message.
///
/// Compose it with existing inputs, selects, checkboxes, or other controls.
/// Set `for` on [`field_label`] to the control's `id`, and connect descriptions
/// and errors through the control's `aria-describedby`. Set `aria-invalid`
/// to `"true"` on an invalid control; the field then colors its label too.
/// A `data-invalid="true"` attribute on the field also colors its label.
/// Validation and the visibility of error messages belong to the caller.
/// Attributes are forwarded to the wrapper and classes are appended.
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

/// A flexible column grouping a field's label, control, or supporting text.
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

/// A [`label`] that follows its field's disabled and invalid states.
///
/// Pass `for` to associate the label with a control's `id`.
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

/// Label-sized text that does not label a control. Use [`field_label`] for that.
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

/// Supporting text. Give it an `id` referenced by the control's `aria-describedby`.
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

/// A decorative divider with optional text between sections of a form.
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

/// An error message for a field, announced when it appears.
///
/// Render it only when there is an error. Give it an `id` referenced by the
/// control's `aria-describedby`, and set `aria-invalid="true"` on the control.
/// Child content can be a message or a list of messages.
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
