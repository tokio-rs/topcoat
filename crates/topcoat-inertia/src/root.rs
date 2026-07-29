use std::fmt::Write as _;
use std::sync::Arc;

use topcoat_core::context::{Cx, app_context};
use topcoat_view::{NodeViewParts, PartsWriter};

use crate::{InertiaConfig, Page};

/// The inert page-data script and empty client mount element.
///
/// This deliberately does not use the legacy `data-page` mount attribute.
/// Inertia.js v3 reads JSON from a sibling `application/json` script instead.
#[derive(Debug, Clone)]
#[must_use]
pub struct InertiaRoot {
    page: Page,
}

impl NodeViewParts for InertiaRoot {
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        let config = app_context::<Arc<InertiaConfig>>(cx);
        let mut json = serde_json::to_string(&self.page)
            .expect("an Inertia page made only of JSON values must serialize");
        json = json.replace('<', "\\u003c");

        let mut html = String::with_capacity(json.len() + 128);
        write!(&mut html, "<script data-page=\"").expect("writing to a String cannot fail");
        escape_attribute(&mut html, &config.root_id);
        html.push_str("\" type=\"application/json\"");
        if let Some(nonce) = config.nonce.as_ref().and_then(|resolve| resolve(cx)) {
            html.push_str(" nonce=\"");
            escape_attribute(&mut html, &nonce);
            html.push('"');
        }
        html.push('>');
        html.push_str(&json);
        html.push_str("</script><div id=\"");
        escape_attribute(&mut html, &config.root_id);
        html.push_str("\"></div>");
        parts.push_str_unescaped(html);
    }
}

/// Builds the v3 bootstrap nodes for `page`.
///
/// Place this exactly once in the root HTML callback passed to
/// [`InertiaConfig::new`]. JSON is escaped so a prop containing `</script>`
/// cannot terminate the inert script element.
pub fn inertia_root(page: &Page) -> InertiaRoot {
    InertiaRoot { page: page.clone() }
}

fn escape_attribute(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '"' => output.push_str("&quot;"),
            _ => output.push(character),
        }
    }
}
