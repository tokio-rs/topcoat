fn main() {
    // Stage the Feather icon set for the `iconify_icon!` references in the
    // component sources.
    topcoat_icon_iconify::BuildConfig::new()
        .icon_set("feather")
        .stage()
        .unwrap();
}
