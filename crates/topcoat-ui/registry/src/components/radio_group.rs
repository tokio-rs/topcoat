use topcoat::{
    Result,
    view::{Attributes, Child, StaticClass, View, class, component, view},
};

/// A group of options with one selection.
///
/// Give every item the same `name` and mark the initial selection `checked`.
/// Attributes are forwarded to the `<div>`, and classes are appended.
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

/// A themed native radio button.
///
/// Pair it with a label. Attributes are forwarded to the `<input>`, while
/// classes are appended to the wrapper.
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
