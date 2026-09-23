use topcoat::{
    Result,
    view::{Attributes, Child, PromotedStr, StaticClass, View, class, component, view},
};

/// How a [`toggle`] relates to the others sharing its `name`.
///
/// [`Default`] is `ToggleKind::Independent`, used when no kind is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ToggleKind {
    /// A toggle that presses and unpresses on its own, like a checkbox.
    #[default]
    Independent,
    /// A toggle of which only one in its group can be pressed, like a radio
    /// button. This is the segmented control: pressing one lets go of the
    /// rest.
    Exclusive,
}

impl ToggleKind {
    /// The `type` of the underlying `<input>`, which is what makes the
    /// browser keep the pressed state this kind calls for.
    fn input_type(self) -> PromotedStr {
        match self {
            Self::Independent => PromotedStr(&"checkbox"),
            Self::Exclusive => PromotedStr(&"radio"),
        }
    }
}

/// The size of a [`toggle`].
///
/// [`Default`] is `ToggleSize::Md`, used when no size is given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ToggleSize {
    /// A compact toggle.
    Sm,
    /// The standard toggle size.
    #[default]
    Md,
    /// A prominent toggle.
    Lg,
}

impl ToggleSize {
    /// The Tailwind classes for this size.
    fn classes(self) -> StaticClass {
        match self {
            Self::Sm => class!("h-8 gap-1.5 rounded-md px-2"),
            Self::Md => class!("h-9 gap-2 rounded-lg px-3"),
            Self::Lg => class!("h-10 gap-2 rounded-lg px-4"),
        }
    }
}

/// The classes shared by every toggle, regardless of size.
const BASE: StaticClass = class!(
    "inline-flex shrink-0 cursor-pointer items-center justify-center border \
     border-transparent text-sm font-medium whitespace-nowrap transition-colors select-none \
     text-muted-foreground hover:bg-foreground/5 hover:text-foreground \
     has-[:checked]:bg-foreground/10 has-[:checked]:text-foreground \
     has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring \
     has-[:focus-visible]:ring-offset-2 has-[:focus-visible]:ring-offset-background \
     has-[:disabled]:pointer-events-none has-[:disabled]:opacity-50",
);

/// A control that stays pressed when selected.
///
/// The native input preserves its state and submits with its form without
/// scripting. Use `kind` to choose independent or exclusive selection. Give
/// exclusive toggles the same `name` to group them.
///
/// Child content supplies the label. Attributes are forwarded to the `<input>`,
/// while classes are appended to the wrapping label.
///
/// ```ignore
/// view! {
///     toggle(
///         attrs: attributes! { name="bold" checked="" },
///         icon(data: iconify_icon!("lucide:bold"), label: "Bold")
///     )
/// }
/// ```
#[component]
pub async fn toggle(
    /// Whether the toggle presses on its own or as one of a group.
    #[default]
    kind: ToggleKind,
    /// The dimensions of the toggle.
    #[default]
    size: ToggleSize,
    /// Extra attributes for the `<input>` element.
    #[default]
    mut attrs: Attributes,
    /// The toggle's content.
    #[default]
    child: Child<'_>,
) -> Result<impl View> {
    // The input is taken out of the layout rather than hidden outright: a
    // `display: none` control is neither focusable nor announced, while an
    // `sr-only` one still takes keyboard focus and reads as the checkbox or
    // radio button it is, named by the label around it.
    Ok(view! {
        <label class=(class!(BASE, size.classes(), attrs.remove("class")))>
            <input type=(kind.input_type()) class="sr-only" (attrs)>
            (child)
        </label>
    })
}

/// A row of related toggles.
///
/// This component controls layout. Give exclusive toggles the same `name` to
/// make them share a selection.
///
/// ```ignore
/// view! {
///     toggle_group(
///         for (value, text) in [("day", "Day"), ("week", "Week")] {
///             toggle(
///                 kind: ToggleKind::Exclusive,
///                 size: ToggleSize::Sm,
///                 attrs: attributes! { name="range" value=(value) },
///                 (text)
///             )
///         }
///     )
/// }
/// ```
#[component]
pub async fn toggle_group(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!(
                "inline-flex w-fit items-center gap-1 rounded-lg border border-border p-1",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    })
}
