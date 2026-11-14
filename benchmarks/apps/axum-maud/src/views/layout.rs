use maud::{DOCTYPE, Markup, html};

use super::{footer, header};
use crate::{
    assets::{SCRIPT_PATH, STYLESHEET_PATH},
    cart::CartCount,
};

/// The document shell shared by every HTML page.
///
/// `title` is the page-specific part of the document title; `None` gives just the store name.
pub fn layout(title: Option<&str>, cart: CartCount, content: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title {
                    @if let Some(title) = title { (title) " | " }
                    "Harbor Market"
                }
                link rel="icon" href="data:,";
                link rel="stylesheet" href=(STYLESHEET_PATH);
                script src=(SCRIPT_PATH) defer {}
            }
            body class="flex min-h-screen flex-col bg-stone-50 text-stone-900 antialiased" {
                a href="#main" class="sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-white focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:shadow" {
                    "Skip to content"
                }
                (header(cart))
                main id="main" class="flex-1" {
                    (content)
                }
                (footer())
            }
        }
    }
}
