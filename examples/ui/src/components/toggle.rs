use topcoat::{
    Result,
    view::{Attributes, PromotedStr, StaticClass, View, class, component, view},
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
    ///
    /// The sizes line up with the button's, so a toggle sits in a row of
    /// buttons without standing out.
    fn classes(self) -> StaticClass {
        match self {
            Self::Sm => class!("h-8 gap-1.5 rounded-md px-2 text-xs"),
            Self::Md => class!("h-9 gap-2 rounded-lg px-3 text-sm"),
            Self::Lg => class!("h-10 gap-2 rounded-lg px-4 text-base"),
        }
    }
}

/// The classes shared by every toggle, regardless of size.
///
/// The state lives in an `<input>` the label wraps, so the label styles
/// itself from the state of the control inside it: tinted while pressed, rung
/// while the control has keyboard focus, and faded while it is disabled.
const BASE: StaticClass = class!(
    "inline-flex shrink-0 cursor-pointer items-center justify-center border \
     border-transparent font-medium whitespace-nowrap transition-colors select-none \
     text-muted-foreground hover:bg-foreground/5 hover:text-foreground \
     has-[:checked]:bg-foreground/10 has-[:checked]:text-foreground \
     has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring \
     has-[:focus-visible]:ring-offset-2 has-[:focus-visible]:ring-offset-background \
     has-[:disabled]:pointer-events-none has-[:disabled]:opacity-50",
);

/// A toggle component: a button that stays pressed.
///
/// The pressed state is the browser's to keep: the toggle is a `<label>`
/// around a hidden `<input>`, so it needs no scripting and submits with the
/// form around it. The `kind` decides which input that is, and so whether the
/// toggle presses on its own or lets go of the others in its group; the
/// `name` among the `attrs` is what forms the group. Child nodes become the
/// toggle's content, and the `attrs` (such as `name`, `value`, `checked`, or
/// `disabled`) are forwarded to the `<input>`; a `class` among them is
/// appended to the label's computed classes.
///
/// ```ignore
/// view! {
///     toggle(
///         attrs: attributes! { name="bold" checked="" },
///         icon(data: iconify_icon!("feather:bold"), label: "Bold")
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
    child: View,
) -> Result {
    // The input is taken out of the layout rather than hidden outright: a
    // `display: none` control is neither focusable nor announced, while an
    // `sr-only` one still takes keyboard focus and reads as the checkbox or
    // radio button it is, named by the label around it.
    view! {
        <label class=(class!(BASE, size.classes(), attrs.remove("class")))>
            <input type=(kind.input_type()) class="sr-only" (attrs)>
            (child)
        </label>
    }
}

/// A row of [`toggle`]s that belong together.
///
/// The group is a rail the toggles sit in, which reads as one control rather
/// than as loose buttons. It only lays them out: what ties exclusive toggles
/// together is still the `name` they share.
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
pub async fn toggle_group(#[default] mut attrs: Attributes, #[default] child: View) -> Result {
    view! {
        <div
            class=(class!(
                "inline-flex w-fit items-center gap-1 rounded-lg border border-border p-1 \
                 shadow-xs",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </div>
    }
}
