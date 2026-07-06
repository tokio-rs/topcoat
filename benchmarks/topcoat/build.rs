fn main() {
    // Only class names in `src/` affect the stylesheet. Without this directive
    // cargo would re-run the script on every build because it writes into the
    // package directory.
    println!("cargo::rerun-if-changed=src");

    // The stylesheet is written to a stable, profile-independent path so that
    // `asset!("assets/tailwind.css")` hashes to the same asset ID in debug and
    // release builds. A bundle produced by `topcoat asset bundle` (which scans
    // a dev build) then also serves a release binary.
    topcoat::tailwind::BuildConfig::new()
        .output("assets/tailwind.css")
        .render()
        .unwrap();
}
