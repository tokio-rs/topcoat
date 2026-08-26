use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// A radio group component: a set of options of which one can be picked.
///
/// The group is a container; what ties its options together is the `name`
/// they share, which is how the browser knows to let go of one when another
/// is picked. Give every [`radio_group_item`] in the group the same `name`,
/// and the one that starts out picked a `checked` attribute. The `attrs`
/// (such as `class`) are forwarded to the underlying `<div>`; a `class` among
/// them is appended to the computed classes.
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
/// The native glyph is suppressed with `appearance-none` so the component can
/// draw its own dot, which keeps the control looking the same across
/// browsers. The circle matches the input control's border and shadow, and
/// picking it recolors the ring rather than filling it, which leaves room for
/// the dot inside.
const RADIO: StaticClass = class!(
    "peer size-4 shrink-0 appearance-none rounded-full border border-border \
     bg-background shadow-xs transition-colors outline-none checked:border-primary \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     focus-visible:ring-offset-background disabled:pointer-events-none",
);

/// The classes for the dot marking the picked option.
const DOT: StaticClass = class!(
    "pointer-events-none absolute inset-0 m-auto size-2 rounded-full bg-primary \
     opacity-0 transition-opacity peer-checked:opacity-100",
);

/// One option of a [`radio_group`]: a themed native `<input type="radio">`.
///
/// The `attrs` (such as `name`, `value`, `checked`, or `disabled`) are
/// forwarded to the `<input>`; a `class` among them is appended to the
/// wrapping element's classes. Pair it with a `label` naming the option.
#[component]
pub async fn radio_group_item(#[default] mut attrs: Attributes) -> Result<impl View> {
    // The dot cannot be drawn by the `<input>` itself, which renders no
    // children or pseudo-elements: it is a sibling overlaid on the control,
    // revealed by the input's `peer` state while picked.
    Ok(view! {
        <span
            class=(class!(
                "relative inline-flex shrink-0 has-[:disabled]:opacity-50",
                attrs.remove("class"),
            ))
        >
            <input type="radio" class=(RADIO) (attrs)>
            <span class=(DOT)></span>
        </span>
    })
}
