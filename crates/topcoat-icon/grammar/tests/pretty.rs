use topcoat_core_grammar::pretty::{Registry, pretty_print_str};
use topcoat_icon_grammar::iconify::{IconifyIcon, Include};

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry
        .register_function_like::<Include>("include")
        .register_function_like::<IconifyIcon>("iconify_icon");
    registry
}

fn format(input: &str) -> String {
    pretty_print_str(&registry(), input).unwrap_or_else(|errors| {
        panic!(
            "failed to format input:\n{input}\nerror: {}",
            errors.first().unwrap()
        )
    })
}

/// Formatting `input` must produce `expected`, and formatting `expected` again
/// must be a fixed point.
fn assert_format(input: &str, expected: &str) {
    assert_eq!(format(input), expected, "formatting:\n{input}");
    assert_eq!(format(expected), expected, "reformatting:\n{expected}");
}

#[test]
fn formats_without_staged_icon_sets() {
    // No build script staged these sets, and formatting must not need them.
    assert_format(
        "include!(pub   \"feather\");",
        "include!(pub \"feather\");",
    );
    assert_format(
        "include!(pub(crate)\"mdi:delete\");",
        "include!(pub(crate) \"mdi:delete\");",
    );
}

#[test]
fn formats_a_single_icon() {
    // `rustfmt` lays out `iconify_icon!("...")`, but not a braced body.
    assert_format(
        "const SUN: IconData = iconify_icon! {   \"lucide:sun\"   };",
        "const SUN: IconData = iconify_icon! { \"lucide:sun\" };",
    );
}

#[test]
fn reports_a_selection_that_is_not_a_single_icon() {
    assert!(pretty_print_str(&registry(), "iconify_icon! { \"lucide:*\" }").is_err());
}
