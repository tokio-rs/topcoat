fn main() {
    // Only class names in `src/` affect the stylesheet.
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=style/tailwind.css");

    // Writes the generated stylesheet to `$OUT_DIR/tailwind.css`; the server
    // embeds it with `include_str!` and serves it at `/tailwind.css`.
    topcoat_tailwind::BuildConfig::new()
        .input("style/tailwind.css")
        .render()
        .unwrap();
}
