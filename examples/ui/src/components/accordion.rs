use topcoat::{
    Result,
    icon::{icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

/// An accordion component: sections that fold away until they are asked for.
///
/// The accordion is a stack of [`accordion_item`]s, each of which opens and
/// closes on its own without scripting. Letting only one stand open at a time
/// takes no more than giving every item the same `name`, which the browser
/// reads as them belonging to one another. The `attrs` (such as `class`) are
/// forwarded to the underlying `<div>`; a `class` among them is appended to
/// the computed classes.
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

/// The classes sliding an [`accordion_item`] open and shut.
///
/// The browser wraps everything after the `<summary>` in a `::details-content`
/// box, which is the part that grows and shrinks: its height runs between
/// zero and the height of the content, with the overflow clipped on the way
/// so the text is revealed rather than squashed. Two rules make that possible.
/// `interpolate-size` is what lets a height land on `auto` and still be
/// animated, since the content's height is not a number the stylesheet knows.
/// The visibility the browser switches along with the state is named in the
/// transition too, with `allow-discrete`: it has no in-between values, so
/// without that it would snap and take the content with it, leaving the
/// height to slide over nothing. Both properties are listed one by one rather
/// than covered by `all`, which does not carry the visibility along.
///
/// Browsers that do not know `::details-content` drop these rules and open
/// and close the section outright, which is what a `<details>` does anyway.
const ANIMATION: StaticClass = class!(
    "[interpolate-size:allow-keywords] [&::details-content]:h-0 \
     [&::details-content]:overflow-hidden \
     [&::details-content]:[transition:height_200ms_ease-out,content-visibility_200ms_allow-discrete] \
     [&[open]::details-content]:h-auto",
);

/// One section of an [`accordion`], holding a trigger and the content it
/// folds away.
///
/// It is built on `<details>`, so it opens and closes on its own; a `name`
/// among the `attrs` puts it in a group where only one section stands open,
/// and an `open` attribute has it start out open. Opening and closing are
/// animated, and while the section is open the `group-open:` variant applies
/// within it, which is what turns the trigger's chevron.
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

/// The row that opens and closes an [`accordion_item`].
///
/// Child nodes become the row's heading. A chevron that turns as the section
/// opens is appended automatically, and the browser's own disclosure marker
/// is taken away in its favor.
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

/// What an [`accordion_item`] folds away, shown while it is open.
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
