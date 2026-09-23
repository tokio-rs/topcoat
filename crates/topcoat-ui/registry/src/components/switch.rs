use topcoat::{
    Result,
    view::{Attributes, StaticClass, View, class, component, view},
};

/// The classes for the native `<input type="checkbox">` that forms the
/// [`switch`] track.
///
/// `appearance-none` hides the native checkbox, and the input is shaped into
/// a pill, so the control looks the same in all browsers. When checked, the
/// track is filled with the primary color.
const SWITCH: StaticClass = class!(
    "peer h-4.5 w-8 shrink-0 appearance-none rounded-full \
     bg-foreground/20 shadow-xs transition-colors outline-none checked:bg-primary \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     focus-visible:ring-offset-background disabled:pointer-events-none",
);

/// The classes for the thumb that slides along the track.
///
/// The thumb is slightly smaller than the track and has the same margin at
/// both ends, so the rim around it looks the same when on and off.
const THUMB: StaticClass = class!(
    "pointer-events-none absolute top-1/2 left-0.5 size-3.5 -translate-y-1/2 \
     rounded-full bg-background shadow-xs transition-transform peer-checked:translate-x-3.5",
);

/// An on/off control for a setting that takes effect right away.
///
/// The control is a native `<input type="checkbox">` with `role="switch"`,
/// so assistive technology announces it as a switch. The `attrs` (such as
/// `name`, `checked`, `disabled`, or event handlers) are forwarded to the
/// `<input>`. A `class` among them is not put on the input but appended to
/// the classes of the `<span>` that wraps it. Set the on state with a
/// `checked` attribute.
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
    // The `<input>` cannot draw the thumb, because it renders no children or
    // pseudo-elements. So the thumb is a sibling placed over the track, and
    // the input's `peer` state slides it to the other end while checked.
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
