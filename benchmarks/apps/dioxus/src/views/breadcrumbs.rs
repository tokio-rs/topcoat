use dioxus::prelude::*;

use super::icons::ChevronRight;
use crate::routes::Route;

/// One link in a breadcrumb trail.
#[derive(Clone, Debug, PartialEq)]
pub struct Crumb {
    pub label: String,
    pub to: Route,
}

impl Crumb {
    pub fn new(label: impl Into<String>, to: Route) -> Self {
        Self {
            label: label.into(),
            to,
        }
    }
}

/// A breadcrumb trail of links ending in the current page.
#[component]
pub fn Breadcrumbs(trail: Vec<Crumb>, current: String) -> Element {
    rsx! {
        nav { aria_label: "Breadcrumb", class: "text-sm",
            ol { class: "flex flex-wrap items-center gap-1.5 text-stone-500",
                for crumb in trail {
                    li { class: "flex items-center gap-1.5",
                        Link { to: crumb.to, class: "hover:text-stone-900", {crumb.label} }
                        ChevronRight {}
                    }
                }
                li {
                    span { aria_current: "page", class: "font-medium text-stone-900", {current} }
                }
            }
        }
    }
}
