use topcoat::{
    Result,
    view::{Attributes, Child, PromotedStr, StaticClass, View, class, component, view},
};

/// How a [`toggle`] behaves together with other toggles with the same
/// `name`.
///
/// The default is `ToggleKind::Independent`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ToggleKind {
    /// A toggle that is pressed and released on its own, like a checkbox.
    #[default]
    Independent,
    /// Only one toggle in the group can be pressed, like a radio button.
    /// Pressing one releases the others, which makes a segmented control.
    Exclusive,
}

impl ToggleKind {
    /// The `type` of the `<input>`, which makes the browser keep the pressed
    /// state that this kind needs.
    fn input_type(self) -> PromotedStr {
        match self {
            Self::Independent => PromotedStr(&"checkbox"),
            Self::Exclusive => PromotedStr(&"radio"),
        }
    }
}

/// The size of a [`toggle`].
///
/// The default is `ToggleSize::Md`.
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
    /// The heights match the button sizes, so a toggle lines up in a row of
    /// buttons.
    fn classes(self) -> StaticClass {
        match self {
            Self::Sm => class!("h-8 gap-1.5 rounded-md px-2"),
            Self::Md => class!("h-9 gap-2 rounded-lg px-3"),
            Self::Lg => class!("h-10 gap-2 rounded-lg px-4"),
        }
    }
}

/// The classes shared by every toggle, regardless of size.
///
/// The state lives in an `<input>` inside the label, and the label styles
/// itself from that state: tinted while pressed, with a focus ring while the
/// input has keyboard focus, and faded while it is disabled.
const BASE: StaticClass = class!(
    "inline-flex shrink-0 cursor-pointer items-center justify-center border \
     border-transparent text-sm font-medium whitespace-nowrap transition-colors select-none \
     text-muted-foreground hover:bg-foreground/5 hover:text-foreground \
     has-[:checked]:bg-foreground/10 has-[:checked]:text-foreground \
     has-[:focus-visible]:ring-2 has-[:focus-visible]:ring-ring \
     has-[:focus-visible]:ring-offset-2 has-[:focus-visible]:ring-offset-background \
     has-[:disabled]:pointer-events-none has-[:disabled]:opacity-50",
);

/// A button that stays pressed until it is pressed again.
///
/// The toggle is a `<label>` around a visually hidden `<input>`. The browser
/// keeps the pressed state, so it works without scripting and is submitted
/// with its form. `kind` decides whether the input is a checkbox or a radio
/// button, and so whether the toggle works on its own or releases the others
/// in its group. Toggles with the same `name` form a group. `kind` defaults
/// to `Independent` and `size` defaults to `Md`.
///
/// Child nodes become the toggle's content. The `attrs` (such as `name`,
/// `value`, `checked`, or `disabled`) are forwarded to the `<input>`. A
/// `class` among them is not put on the input but appended to the classes of
/// the `<label>`.
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
    // The input is visually hidden with `sr-only` instead of `display: none`.
    // A `display: none` control cannot be focused or announced. An `sr-only`
    // control still takes keyboard focus and is announced as a checkbox or
    // radio button, named by the label around it.
    Ok(view! {
        <label class=(class!(BASE, size.classes(), attrs.remove("class")))>
            <input type=(kind.input_type()) class="sr-only" (attrs)>
            (child)
        </label>
    })
}

/// A row of [`toggle`]s that belong together.
///
/// The toggles sit in a bordered box, so they look like one control. The
/// group only handles the layout. Exclusive toggles still need the same
/// `name` to work as a group. The `attrs` are forwarded to the `<div>`, and a
/// `class` among them is appended to the component's classes.
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
