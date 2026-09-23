use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// A set of options from which the user can pick one.
///
/// The group is a `<div>` with `role="radiogroup"`. The browser groups the
/// options by their shared `name`, so give every [`radio_group_item`] in the
/// group the same `name`. Give the option that starts selected a `checked`
/// attribute. The `attrs` (such as `class`) are forwarded to the `<div>`. A
/// `class` among them is appended to the component's classes.
///
/// ```ignore
/// view! {
///     radio_group(
///         for (value, text) in [("weekly", "Weekly"), ("monthly", "Monthly")] {
///             <div class="flex items-center gap-2">
///                 radio_group_item(
///                     attrs: attributes! { id=(value) name="billing" value=(value) }
///                 )
///                 label(attrs: attributes! { for=(value) }, (text))
///             </div>
///         }
///     )
/// }
/// ```
#[component]
pub async fn radio_group(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            role="radiogroup"
            class=(class!("grid gap-3", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}

/// The classes for the native `<input type="radio">` inside a
/// [`radio_group_item`].
///
/// `appearance-none` hides the native dot, so the component can draw its own
/// and look the same in all browsers. The circle has the same border as the
/// input control. When selected, only the border changes color, which leaves
/// room for the dot inside.
const RADIO: StaticClass = class!(
    "peer size-4 shrink-0 appearance-none rounded-full border border-border \
     bg-background transition-colors outline-none checked:border-primary \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     focus-visible:ring-offset-background disabled:pointer-events-none",
);

/// The classes for the dot that marks the selected option.
const DOT: StaticClass = class!(
    "pointer-events-none absolute inset-0 m-auto size-2 rounded-full bg-primary \
     opacity-0 transition-opacity peer-checked:opacity-100",
);

/// One option of a [`radio_group`], rendered as a styled native
/// `<input type="radio">`.
///
/// The `attrs` (such as `name`, `value`, `checked`, or `disabled`) are
/// forwarded to the `<input>`. A `class` among them is not put on the input
/// but appended to the classes of the `<span>` that wraps it. Add a `label`
/// that names the option.
#[component]
pub async fn radio_group_item(#[default] mut attrs: Attributes) -> Result<impl View> {
    // The `<input>` cannot draw the dot, because it renders no children or
    // pseudo-elements. So the dot is a sibling placed over the control, and
    // the input's `peer` state shows it while selected.
    Ok(view! {
        <span
            class=(class!(
                "peer relative inline-flex shrink-0 has-[:disabled]:opacity-50",
                attrs.remove("class"),
            ))
        >
            <input type="radio" class=(RADIO) (attrs)>
            <span class=(DOT)></span>
        </span>
    })
}
