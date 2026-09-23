use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// The classes for the native `<input type="checkbox">` serving as the
/// [`switch`] track.
const SWITCH: StaticClass = class!(
    "peer h-4.5 w-8 shrink-0 appearance-none rounded-full \
     bg-foreground/20 shadow-xs transition-colors outline-none checked:bg-primary \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     focus-visible:ring-offset-background disabled:pointer-events-none",
);

/// The classes for the thumb sliding along the track.
const THUMB: StaticClass = class!(
    "pointer-events-none absolute top-1/2 left-0.5 size-3.5 -translate-y-1/2 \
     rounded-full bg-background shadow-xs transition-transform peer-checked:translate-x-3.5",
);

/// An on/off control for a setting that applies immediately.
///
/// Uses a native checkbox with the `switch` role. Pass `checked` for its initial
/// state. Attributes are forwarded to the `<input>`, while classes are appended
/// to the wrapper.
///
/// ```ignore
/// view! {
///     <div class="flex items-center gap-2">
///         switch(attrs: attributes! { id="airplane-mode" checked="" })
///         label(attrs: attributes! { for="airplane-mode" }, "Airplane mode")
///     </div>
/// }
/// ```
#[component]
pub async fn switch(#[default] mut attrs: Attributes) -> Result<impl View> {
    // The thumb cannot be drawn by the `<input>` itself, which renders no
    // children or pseudo-elements: it is a sibling overlaid on the track,
    // slid to the far end by the input's `peer` state while checked.
    Ok(view! {
        <span
            class=(class!(
                "peer relative inline-flex shrink-0 has-[:disabled]:opacity-50",
                attrs.remove("class"),
            ))
        >
            <input type="checkbox" role="switch" class=(SWITCH) (attrs)>
            <span class=(THUMB)></span>
        </span>
    })
}
