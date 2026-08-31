fn main() {
    // Stage the Lucide icon set for the `iconify_icon!` references in the
    // `#[cfg(test)]` component sources. Gated behind the `stage-icons` feature
    // so ordinary builds and docs.rs stay offline; see the feature in
    // `Cargo.toml`.
    #[cfg(feature = "stage-icons")]
    topcoat_icon::iconify::BuildConfig::new()
        .icon_set("lucide")
        .stage()
        .unwrap();
}
