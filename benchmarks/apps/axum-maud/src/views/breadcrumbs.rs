use maud::{Markup, html};

use super::icons;

/// A breadcrumb trail of `(label, href)` links ending in the current page.
pub fn breadcrumbs(trail: &[(&str, &str)], current: &str) -> Markup {
    html! {
        nav aria-label="Breadcrumb" class="text-sm" {
            ol class="flex flex-wrap items-center gap-1.5 text-stone-500" {
                @for (label, href) in trail {
                    li class="flex items-center gap-1.5" {
                        a href=(href) class="hover:text-stone-900" { (label) }
                        (icons::chevron_right())
                    }
                }
                li {
                    span aria-current="page" class="font-medium text-stone-900" { (current) }
                }
            }
        }
    }
}
