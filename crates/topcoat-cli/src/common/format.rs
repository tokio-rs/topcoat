//! Formatting of Topcoat macro bodies.

use topcoat_core_grammar::pretty::Registry;

/// Every macro whose body Topcoat can format: function-like macros, then
/// attribute macros.
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
    "include",
    "iconify_icon",
    "mail",
    "messages",
    "t",
    "not_found",
    "segment",
    "path_param",
    "module_param",
    "expr",
    "memoize",
    "component",
    "page",
    "route",
    "layout",
    "layer",
    "query_params",
    "procedure",
    "shard",
    "record",
];

/// Registers the macro called `name` for formatting. Returns `false` if it is not one of
/// [`MACROS`].
pub fn register(registry: &mut Registry, name: &str) -> bool {
    use topcoat_core_grammar::memoize::MemoizeAttr;
    use topcoat_font_grammar::{font::Font, font_face::FontFace, fontsource};
    use topcoat_icon_grammar::iconify::{IconifyIcon, Include};
    use topcoat_l10n_grammar::fluent::{Messages, Translate};
    use topcoat_mail_grammar::mail::Mail;
    use topcoat_router_grammar::{
        layer::LayerAttr, layout::LayoutAttr, module_param::ModuleParam, not_found::NotFound,
        page::PageAttr, path_param::PathParam, query_params::QueryParamsAttr, route::RouteAttr,
        segment::Segment,
    };
    use topcoat_runtime_grammar::{
        expr::ExprInput, procedure::ProcedureAttr, record::RecordAttr, shard::ShardAttr,
    };
    use topcoat_view_grammar::{
        attributes::Attributes,
        class::Class,
        component::ComponentAttr,
        live::{Emit, Live},
        view::View,
    };

    match name {
        "view" => registry.register_function_like::<View>(name),
        "attributes" => registry.register_function_like::<Attributes>(name),
        "class" => registry.register_function_like::<Class>(name),
        "live" => registry.register_function_like::<Live>(name),
        "emit" => registry.register_function_like::<Emit>(name),
        "font_face" => registry.register_function_like::<FontFace>(name),
        "font" => registry.register_function_like::<Font>(name),
        "fontsource_font_face" => {
            registry.register_function_like::<fontsource::font_face::FontsourceFontFace>(name)
        }
        "fontsource_font" => registry.register_function_like::<fontsource::font::FontsourceFont>(name),
        "include" => registry.register_function_like::<Include>(name),
        "iconify_icon" => registry.register_function_like::<IconifyIcon>(name),
        "mail" => registry.register_function_like::<Mail>(name),
        "messages" => registry.register_function_like::<Messages>(name),
        "t" => registry.register_function_like::<Translate>(name),
        "not_found" => registry.register_function_like::<NotFound>(name),
        "segment" => registry.register_function_like::<Segment>(name),
        "path_param" => registry.register_function_like::<PathParam>(name),
        "module_param" => registry.register_function_like::<ModuleParam>(name),
        "expr" => registry.register_function_like::<ExprInput>(name),
        "memoize" => registry.register_attribute::<MemoizeAttr>(name),
        "component" => registry.register_attribute::<ComponentAttr>(name),
        "page" => registry.register_attribute::<PageAttr>(name),
        "route" => registry.register_attribute::<RouteAttr>(name),
        "layout" => registry.register_attribute::<LayoutAttr>(name),
        "layer" => registry.register_attribute::<LayerAttr>(name),
        "query_params" => registry.register_attribute::<QueryParamsAttr>(name),
        "procedure" => registry.register_attribute::<ProcedureAttr>(name),
        "shard" => registry.register_attribute::<ShardAttr>(name),
        "record" => registry.register_attribute::<RecordAttr>(name),
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
