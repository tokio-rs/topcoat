//! Formatting of Topcoat macro bodies.

use topcoat_core_grammar::pretty::Registry;

/// Every macro whose body Topcoat can format.
pub const MACROS: &[&str] = &[
    "view",
    "attributes",
    "class",
    "live",
    "emit",
    "font_face",
    "font",
    "fontsource_font_face",
    "fontsource_font",
    "mail",
];

/// Registers the macro called `name` for formatting. Returns `false` if it is not one of
/// [`MACROS`].
pub fn register(registry: &mut Registry, name: &str) -> bool {
    use topcoat_font_grammar::{font::Font, font_face::FontFace, fontsource};
    use topcoat_mail_grammar::mail::Mail;
    use topcoat_view_grammar::{
        attributes::Attributes,
        class::Class,
        live::{Emit, Live},
        view::View,
    };

    match name {
        "view" => registry.register_macro::<View>(name),
        "attributes" => registry.register_macro::<Attributes>(name),
        "class" => registry.register_macro::<Class>(name),
        "live" => registry.register_macro::<Live>(name),
        "emit" => registry.register_macro::<Emit>(name),
        "font_face" => registry.register_macro::<FontFace>(name),
        "font" => registry.register_macro::<Font>(name),
        "fontsource_font_face" => {
            registry.register_macro::<fontsource::font_face::FontsourceFontFace>(name)
        }
        "fontsource_font" => registry.register_macro::<fontsource::font::FontsourceFont>(name),
        "mail" => registry.register_macro::<Mail>(name),
        _ => return false,
    };
    true
}

/// Returns a registry that formats every macro in [`MACROS`].
pub fn registry() -> Registry {
    let mut registry = Registry::new();
    for name in MACROS {
        register(&mut registry, name);
    }
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_macro_registers() {
        let mut registry = Registry::new();
        for name in MACROS {
            assert!(register(&mut registry, name), "{name}");
        }
        assert!(!register(&mut registry, "println"));
    }
}
