use topcoat::{
    asset::{Asset, asset, asset_config},
    context::Cx,
    inertia::{Page, inertia_root},
    view::{HtmlContext, NodeViewParts, PartsWriter, View, ViewParts},
};

pub const APP_JS: Asset = asset!("../assets/app.js");
pub const APP_CSS: Asset = asset!("../assets/app.css");

pub fn root(cx: &Cx, page: &Page) -> View {
    let css = asset_config(cx).resolve(APP_CSS);
    let javascript = asset_config(cx).resolve(APP_JS);
    let mut parts = ViewParts::new();

    PartsWriter::new(&mut parts, HtmlContext::Text).push_str_unescaped(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><link rel=\"stylesheet\" href=\"",
    );
    PartsWriter::new(&mut parts, HtmlContext::AttributeValue).push_str(css);
    PartsWriter::new(&mut parts, HtmlContext::Text).push_str_unescaped("\"></head><body>");
    inertia_root(page).into_view_parts(cx, &mut PartsWriter::new(&mut parts, HtmlContext::Text));
    PartsWriter::new(&mut parts, HtmlContext::Text)
        .push_str_unescaped("<script type=\"module\" src=\"");
    PartsWriter::new(&mut parts, HtmlContext::AttributeValue).push_str(javascript);
    PartsWriter::new(&mut parts, HtmlContext::Text)
        .push_str_unescaped("\"></script></body></html>");

    View::new(parts)
}
