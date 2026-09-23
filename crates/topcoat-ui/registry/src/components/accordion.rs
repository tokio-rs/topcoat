use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

/// A stack of sections that the user can open and close.
///
/// The accordion holds [`accordion_item`]s. Each item opens and closes
/// without scripting. To allow only one open item at a time, give every item
/// the same `name` attribute. The browser then treats them as one group.
///
/// The `attrs` (such as `class`) are forwarded to the `<div>`. A `class`
/// among them is appended to the component's classes. The same holds for the
/// other accordion components.
///
/// ```ignore
/// view! {
///     accordion(
///         for (question, answer) in questions {
///             // The shared name is what closes the open item when
///             // another is opened. Leave it out to let several stand
///             // open at once.
///             accordion_item(
///                 attrs: attributes! { name="faq" },
///                 accordion_trigger((question))
///                 accordion_content((answer))
///             )
///         }
///     )
/// }
/// ```
#[component]
pub async fn accordion(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div class=(class!("w-full", attrs.remove("class"))) (attrs)>(child)</div>
    })
}

/// The classes that animate an [`accordion_item`] opening and closing.
///
/// The browser wraps everything after the `<summary>` in a
/// `::details-content` box. Its height animates between zero and the height
/// of the content, and the overflow is clipped, so the text is revealed
/// instead of squashed. `interpolate-size` allows the height to animate to
/// `auto`. The transition also lists `content-visibility` with
/// `allow-discrete`. Without it, the content would disappear at once when the
/// item closes. Both properties are listed by name, because `all` does not
/// include `content-visibility`.
///
/// Browsers without `::details-content` ignore these rules and open and close
/// the item without animation.
const ANIMATION: StaticClass = class!(
    "[interpolate-size:allow-keywords] [&::details-content]:h-0 \
     [&::details-content]:overflow-hidden \
     [&::details-content]:[transition:height_200ms_ease-out,content-visibility_200ms_allow-discrete] \
     [&[open]::details-content]:h-auto",
);

/// One section of an [`accordion`], holding an [`accordion_trigger`] and an
/// [`accordion_content`].
///
/// It is a `<details>` element, so it opens and closes without scripting. A
/// `name` attribute puts it in a group in which only one item can be open,
/// and an `open` attribute makes it start open. Opening and closing are
/// animated. While the item is open, the `group-open:` variant applies to its
/// content, which rotates the trigger's chevron.
#[component]
pub async fn accordion_item(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <details
            class=(class!(
                "group border-b border-border last:border-b-0",
                ANIMATION,
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
        </details>
    })
}

/// The row that opens and closes an [`accordion_item`], rendered as a
/// `<summary>`.
///
/// Child nodes become the row's heading. A chevron is added after them and
/// rotates when the item opens. The browser's default marker is hidden.
#[component]
pub async fn accordion_trigger(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <summary
            class=(class!(
                "flex w-full cursor-pointer list-none items-center justify-between gap-4 py-4 \
                 text-left text-sm font-medium outline-none transition-colors \
                 hover:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring \
                 focus-visible:ring-offset-2 focus-visible:ring-offset-background \
                 [&::-webkit-details-marker]:hidden",
                attrs.remove("class"),
            ))
            (attrs)
        >
            (child)
            icon(
                data: iconify_icon!("lucide:chevron-down"),
                attrs: attributes! {
                    class="size-4 shrink-0 text-muted-foreground transition-transform \
                        duration-200 ease-out group-open:rotate-180"
                }
            )
        </summary>
    })
}

/// The content of an [`accordion_item`], shown while the item is open.
#[component]
pub async fn accordion_content(
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    Ok(view! {
        <div
            class=(class!("pb-4 text-sm text-muted-foreground", attrs.remove("class")))
            (attrs)
        >
            (child)
        </div>
    })
}
