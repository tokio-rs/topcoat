fn main() {
    // The installed theme stylesheet is the Tailwind input: it carries the
    // `@import "tailwindcss"` directive and the theme's design tokens.
    topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .render()
        .unwrap();

    // Stage the Feather icon set for the `iconify_icon!` references in main.rs.
    topcoat::icon::iconify::BuildConfig::new()
        .icon_set("feather")
        .stage()
        .unwrap();
}
