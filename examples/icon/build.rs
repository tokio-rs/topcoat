fn main() {
    // Download and stage the Feather icon set used by the application.
    topcoat::icon::iconify::BuildConfig::new()
        .icon_set("feather")
        .stage()
        .unwrap();
}
