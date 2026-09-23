use topcoat::{
    Result,
    context::Cx,
    icon::{IconData, icon, iconify::iconify_icon},
    view::{Attributes, Child, StaticClass, View, attributes, class, component, view},
};

/// The classes for the native `<select>` inside the [`select`] component.
const SELECT: StaticClass = class!(
    "h-9 w-full appearance-none items-center rounded-lg border border-border \
     bg-transparent pr-8 pl-3 text-left text-sm transition-colors outline-none \
     focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 \
     aria-invalid:border-destructive aria-invalid:focus-visible:ring-destructive \
     focus-visible:ring-offset-background disabled:pointer-events-none",
);

/// The classes restyling the drop-down picker, for browsers that support
/// customizable selects (`appearance: base-select`, set on the `<select>` by
/// the component's wrapper).
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

/// The icon marking the picker's checked option.
const CHECKMARK: IconData = iconify_icon!("lucide:check");

/// The inline style for the [`select`] wrapper, carrying [`CHECKMARK`] as a
/// data URI in the `--select-checkmark` custom property. The indirection
/// exists because the `::checkmark` pseudo-element can only take the icon
/// through a stylesheet, as a mask image, while the icon's markup is only
/// available here.
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

/// A themed native select control.
///
/// Provide `<option>` and `<optgroup>` children. Attributes are forwarded to
/// the `<select>`, while classes are appended to the wrapper. It fills its
/// container by default. Set `aria-invalid="true"` to show its error styling.
///
/// Browsers with customizable select support use a styled picker. Others use
/// the native picker. For a styled group heading, put a `<legend>` first in the
/// `<optgroup>` and retain its `label` attribute for the native fallback.
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
    // `appearance: base-select` opts into the customizable picker. It is set
    // from the wrapper because the descendant selector outranks the
    // `appearance-none` fallback in specificity, making the outcome
    // independent of stylesheet order; browsers without support drop the
    // invalid declaration and keep the fallback.
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
