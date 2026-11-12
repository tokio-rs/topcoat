fn main() {
    topcoat::tailwind::BuildConfig::new()
        // The benchmark shell's Tailwind CLI, so every app compiles its stylesheet with the same
        // release.
        .executable("tailwindcss")
        .input("styles.css")
        // A fixed path instead of `OUT_DIR`, whose name changes with the enabled cargo
        // features. See `STYLESHEET` in `src/app.rs`.
        .output("target/tailwind.css")
        .render()
        .unwrap();
}
