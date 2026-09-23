use topcoat::{
    Result,
    context::Cx,
    icon::{IconData, icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

/// The classes for the native `<select>` inside the [`select`] component.
///
/// It has the same size as the input control. The native arrow is hidden, so
/// the component can draw its own chevron and look the same in all browsers.
/// The extra right padding leaves room for the chevron.
const SELECT: StaticClass = class!(
    "h-9 w-full appearance-none items-center rounded-lg border border-border \
     bg-transparent pr-8 pl-3 text-left text-sm transition-colors outline-none \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     aria-invalid:border-destructive aria-invalid:focus-visible:ring-destructive \
     focus-visible:ring-offset-background disabled:pointer-events-none",
);

/// The classes that style the drop-down list, in browsers that support
/// customizable selects (`appearance: base-select`, which the wrapper sets on
/// the `<select>`).
///
/// The list and its options look like the dropdown menu's panel and items:
/// the same raised surface and the same hover and focus tints. The selected
/// option has a checkmark at its right edge. The checkmark is the
/// [`CHECKMARK`] icon used as a mask over the muted foreground color (see
/// [`checkmark_style`]). The browser's own picker icon is hidden in favor of
/// the component's chevron. Browsers without support ignore these rules and
/// show the operating system's list.
const PICKER: StaticClass = class!(
    "[&::picker(select)]:[appearance:base-select] \
     [&::picker(select)]:mt-1 [&::picker(select)]:rounded-lg \
     [&::picker(select)]:border [&::picker(select)]:border-border \
     [&::picker(select)]:bg-popover [&::picker(select)]:p-1 \
     [&::picker(select)]:text-popover-foreground [&::picker(select)]:shadow-sm \
     [&::picker-icon]:hidden \
     [&_optgroup>legend]:px-2 [&_optgroup>legend]:py-1.5 \
     [&_optgroup>legend]:text-xs [&_optgroup>legend]:font-medium \
     [&_optgroup>legend]:text-muted-foreground [&_optgroup>legend]:cursor-default \
     [&_optgroup>legend]:select-none \
     [&_option]:flex [&_option]:items-center [&_option]:gap-2 [&_option]:rounded-md \
     [&_option]:px-2 [&_option]:py-1.5 [&_option]:text-sm [&_option]:outline-none \
     [&_option:hover]:bg-foreground/5 [&_option:focus]:bg-foreground/5 \
     [&_option:checked]:font-medium \
     [&_option::checkmark]:order-1 [&_option::checkmark]:ml-auto \
     [&_option::checkmark]:size-4 [&_option::checkmark]:shrink-0 \
     [&_option::checkmark]:content-[''] [&_option::checkmark]:bg-muted-foreground \
     [&_option::checkmark]:[mask-size:100%_100%] \
     [&_option::checkmark]:[mask-image:var(--select-checkmark)]",
);

/// The icon that marks the selected option in the list.
const CHECKMARK: IconData = iconify_icon!("lucide:check");

/// The inline style for the [`select`] wrapper. It sets the
/// `--select-checkmark` custom property to [`CHECKMARK`] as a data URI. The
/// `::checkmark` pseudo-element can only get the icon from CSS, as a mask
/// image, and the icon's markup is only available here.
fn checkmark_style(cx: &Cx) -> String {
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{}">{}</svg>"#,
        CHECKMARK.view_box(),
        CHECKMARK.into_body().render(cx),
    );
    let mut style = String::from(r#"--select-checkmark: url("data:image/svg+xml,"#);
    // Percent-encode the characters that cannot appear in a double-quoted
    // CSS url().
    for char in svg.chars() {
        match char {
            '%' => style.push_str("%25"),
            '"' => style.push_str("%22"),
            '#' => style.push_str("%23"),
            _ => style.push(char),
        }
    }
    style.push_str(r#"")"#);
    style
}

/// A drop-down list, rendered as a styled native `<select>`.
///
/// Child nodes become the content of the `<select>`, usually `<option>` and
/// `<optgroup>` elements. The `attrs` (such as `name`, `disabled`, or event
/// handlers) are forwarded to the `<select>`. A `class` among them is not put
/// on the `<select>` but appended to the classes of the `<span>` that wraps
/// it, so width classes size the whole control. Like the input, it fills the
/// width of its container by default. Set `aria-invalid="true"` to show the
/// error border and focus ring.
///
/// For a styled group heading, add a `<legend>` as the first child of an
/// `<optgroup>`. Keep the `label` attribute of the `<optgroup>` for browsers
/// that show the native list.
///
/// In browsers that support customizable selects, the list looks like the
/// dropdown menu component, and the chevron flips while it is open. Other
/// browsers show the operating system's list. The closed control looks the
/// same everywhere.
///
/// ```ignore
/// view! {
///     select(
///         attrs: attributes! { name="region" },
///         <option>"eu-central-1"</option>
///         <option>"us-east-1"</option>
///     )
/// }
/// ```
#[component]
pub async fn select(
    cx: &Cx,
    #[default] mut attrs: Attributes,
    #[default] child: Child<'_>,
) -> Result<impl View> {
    // `appearance: base-select` turns on the customizable list. It is set
    // from the wrapper because the descendant selector has a higher
    // specificity than the `appearance-none` fallback, so stylesheet order
    // does not matter. Browsers without support drop the invalid declaration
    // and keep the fallback.
    Ok(view! {
        <span
            class=(class!(
                "relative block has-[:disabled]:opacity-50 \
                 [&>select]:[appearance:base-select] \
                 [&:has(select:open)>svg]:rotate-180",
                attrs.remove("class"),
            ))
            style=(checkmark_style(cx))
        >
            <select class=(class!(SELECT, PICKER)) (attrs)>(child)</select>
            icon(
                data: iconify_icon!("lucide:chevron-down"),
                attrs: attributes! {
                    class="pointer-events-none absolute top-1/2 right-3 size-4 \
                        -translate-y-1/2 text-muted-foreground transition-transform"
                }
            )
        </span>
    })
}
