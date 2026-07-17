use topcoat::{
    Result,
    view::{Attributes, class, component, view},
};

/// The classes for the native `<input type="checkbox">` serving as the
/// [`switch`] track.
///
/// The native glyph is suppressed with `appearance-none` and the input is
/// stretched into a pill-shaped track, which keeps the control looking the
/// same across browsers. Checking it fills the track with the primary color.
const SWITCH: &str = "peer h-4.5 w-8 shrink-0 appearance-none rounded-full \
    bg-foreground/20 shadow-xs transition-colors outline-none checked:bg-primary \
    focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
    focus-visible:ring-offset-background disabled:pointer-events-none";

/// The classes for the thumb sliding along the track.
///
/// The thumb is two spacing units smaller than the track, inset by one unit
/// at rest, and travels to the mirrored position when checked, so the rim
/// around it is uniform at both ends of the track.
const THUMB: &str = "pointer-events-none absolute top-1/2 left-0.5 size-3.5 -translate-y-1/2 \
    rounded-full bg-background shadow-xs transition-transform peer-checked:translate-x-3.5";

/// A switch component: an on/off toggle for a setting that applies
/// immediately.
///
/// The control is a native `<input type="checkbox">` carrying the `switch`
/// role, so assistive technology announces it as a switch. The `attrs` (such
/// as `name`, `checked`, `disabled`, or event handlers) are forwarded to the
/// `<input>`; a `class` among them is appended to the wrapping element's
/// classes. Set the on state with a plain `checked` attribute.
///
/// ```rust
/// view! {
///     <div class="flex items-center gap-2">
///         switch(attrs: attributes! { id="airplane-mode" checked=(true) })
///         label(attrs: attributes! { for="airplane-mode" }, "Airplane mode")
///     </div>
/// }
/// ```
#[component]
pub async fn switch(#[default] mut attrs: Attributes) -> Result {
    // The thumb cannot be drawn by the `<input>` itself, which renders no
    // children or pseudo-elements: it is a sibling overlaid on the track,
    // slid to the far end by the input's `peer` state while checked.
    view! {
        <span
            class=(class!(
                "relative inline-flex shrink-0 has-[:disabled]:opacity-50",
                attrs.remove("class"),
            ))
        >
            <input type="checkbox" role="switch" class=(SWITCH) (attrs)>
            <span class=(THUMB)></span>
        </span>
    }
}
