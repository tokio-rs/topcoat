//! Inline SVG icons. All are decorative and hidden from assistive technology.

use dioxus::prelude::*;

/// The Harbor Market logo mark.
#[component]
pub fn Logo() -> Element {
    rsx! {
        svg { view_box: "0 0 32 32", "aria-hidden": "true", class: "size-8",
            rect { width: "32", height: "32", rx: "9", class: "fill-teal-700" }
            path {
                d: "M16 6v10M12 10h8M7 20c3 0 3-2.5 6-2.5s3 2.5 6 2.5 3-2.5 6-2.5M7 25c3 0 3-2.5 6-2.5s3 2.5 6 2.5 3-2.5 6-2.5",
                fill: "none",
                stroke: "white",
                stroke_width: "2",
                stroke_linecap: "round",
            }
        }
    }
}

#[component]
pub fn Bag() -> Element {
    rsx! { Outline { class: "size-5", path: "M6 8h12l-1 12H7L6 8ZM9 8a3 3 0 0 1 6 0" } }
}

#[component]
pub fn Menu() -> Element {
    rsx! { Outline { class: "size-6", path: "M4 6h16M4 12h16M4 18h16" } }
}

#[component]
pub fn ChevronRight() -> Element {
    rsx! { Outline { class: "size-4 text-stone-300", path: "m9 6 6 6-6 6" } }
}

#[component]
pub fn ArrowLeft() -> Element {
    rsx! { Outline { class: "size-4", path: "M19 12H5m6-6-6 6 6 6" } }
}

#[component]
pub fn ArrowRight() -> Element {
    rsx! { Outline { class: "size-4", path: "M5 12h14m-6-6 6 6-6 6" } }
}

#[component]
pub fn Minus() -> Element {
    rsx! { Outline { class: "size-4", path: "M5 12h14" } }
}

#[component]
pub fn Plus() -> Element {
    rsx! { Outline { class: "size-4", path: "M12 5v14M5 12h14" } }
}

#[component]
pub fn Check() -> Element {
    rsx! { Outline { class: "size-5 text-teal-700", path: "m5 12 5 5L20 7" } }
}

#[component]
pub fn Truck() -> Element {
    rsx! {
        Outline {
            class: "size-6",
            path: "M3 6h11v10H3zM14 10h4l3 3v3h-7M7 19a2 2 0 1 0 0-4 2 2 0 0 0 0 4ZM17 19a2 2 0 1 0 0-4 2 2 0 0 0 0 4Z",
        }
    }
}

#[component]
pub fn Refresh() -> Element {
    rsx! {
        Outline {
            class: "size-6",
            path: "M4 12a8 8 0 0 1 14-5.3L20 9M20 4v5h-5M20 12a8 8 0 0 1-14 5.3L4 15M4 20v-5h5",
        }
    }
}

#[component]
pub fn Shield() -> Element {
    rsx! {
        Outline {
            class: "size-6",
            path: "M12 3 5 6v6c0 4.5 3 7.5 7 9 4-1.5 7-4.5 7-9V6l-7-3Zm-3 9 2 2 4-4",
        }
    }
}

/// A 24x24 stroked icon with a single path.
#[component]
fn Outline(class: &'static str, path: &'static str) -> Element {
    rsx! {
        svg {
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            class,
            path { d: path }
        }
    }
}
