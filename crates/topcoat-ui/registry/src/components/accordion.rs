use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

/// A group of collapsible sections.
///
/// Add an `accordion_item` for each section. Items open and close without JavaScript.
/// Give them the same `name` attribute to allow only one open item at a time.
///
/// `attrs` are forwarded to the outer `<div>`. Extra classes are added to its classes.
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

/// Classes that animate the section height when it opens or closes.
///
/// The transition includes `content-visibility` with `allow-discrete` to keep the
/// content visible during animation. Browsers without `::details-content` support use
/// the native disclosure behavior.
const ANIMATION: StaticClass = class!(
    "[interpolate-size:allow-keywords] [&::details-content]:h-0 \
     [&::details-content]:overflow-hidden \
     [&::details-content]:[transition:height_200ms_ease-out,content-visibility_200ms_allow-discrete] \
     [&[open]::details-content]:h-auto",
);

/// A collapsible section with a trigger and content.
///
/// Uses a native `<details>` element. Pass `open` in `attrs` to open it initially.
/// Items with the same `name` attribute form a group in which only one item can be
/// open.
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

/// The heading that opens and closes an accordion section.
///
/// Pass the heading as child content. A chevron shows whether the section is open.
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

/// The content shown when an accordion section is open.
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
