#![doc = include_str!("../docs/view.md")]

#[cfg(feature = "asset")]
use topcoat_asset::{Asset, asset};
pub use topcoat_view::*;
pub use topcoat_view_macro::*;

#[cfg(feature = "asset")]
const DEFER_SCRIPT: Asset = asset!("browser/defer.js", rename: "topcoat-defer");

/// Renders the external browser helper that applies streamed deferred views.
///
/// Place this in the document head. It is intentionally parser-blocking so it
/// can observe deferred fragments while the rest of the response streams.
#[cfg(feature = "asset")]
#[topcoat::view::component]
pub async fn defer_script() -> topcoat::Result {
    topcoat::view::view! { <script src=(DEFER_SCRIPT)></script> }
}

#[cfg(all(test, feature = "asset"))]
mod tests {
    use topcoat_asset::{AssetConfig, Manifest};
    use topcoat_core::context::CxTestBuilder;
    use topcoat_view::Component;

    use super::*;

    #[tokio::test]
    async fn defer_script_runs_while_the_document_is_parsing() {
        let manifest = Manifest::parse(&format!(
            r#"
version = 1

[[assets]]
id = {}
file = "topcoat-defer.js"
hash = "0"
content_type = "text/javascript"
"#,
            DEFER_SCRIPT.id().as_u64()
        ))
        .unwrap();
        let cx = CxTestBuilder::new()
            .app_context(AssetConfig::hosted_at("https://example.com", manifest))
            .build();
        let props = defer_script::props_builder().build();
        #[allow(clippy::default_constructed_unit_structs)]
        let view = Component::render(defer_script::default(), &cx, props)
            .await
            .unwrap();
        let html = view.render(&cx);

        assert!(html.starts_with("<script src="), "{html}");
        assert!(!html.contains("type=\"module\""), "{html}");
    }
}
