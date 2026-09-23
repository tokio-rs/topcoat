use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// A container for options that allow one selection.
///
/// Give each `radio_group_item` the same `name` attribute to form the selection group.
/// Set `checked` on the initially selected item. `attrs` are forwarded to the outer
/// `<div>`, with extra classes added to its classes.
/// Name the group with `aria-label` or `aria-labelledby` in `attrs`.
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

/// Classes for the radio input and its selected border.
const RADIO: StaticClass = class!(
    "peer size-4 shrink-0 appearance-none rounded-full border border-border \
     bg-background transition-colors outline-none checked:border-primary \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     focus-visible:ring-offset-background disabled:pointer-events-none",
);

/// The classes for the dot marking the picked option.
const DOT: StaticClass = class!(
    "pointer-events-none absolute inset-0 m-auto size-2 rounded-full bg-primary \
     opacity-0 transition-opacity peer-checked:opacity-100",
);

/// A native radio input styled as a group option.
///
/// Pair it with a label. Classes in `attrs` apply to the wrapper, while other
/// attributes go on the `<input>`.
#[component]
pub async fn radio_group_item(#[default] mut attrs: Attributes) -> Result<impl View> {
    // The dot cannot be drawn by the `<input>` itself, which renders no
    // children or pseudo-elements: it is a sibling overlaid on the control,
    // revealed by the input's `peer` state while picked.
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
