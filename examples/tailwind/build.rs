fn main() {
    // Scan the project for Tailwind classes and generate the stylesheet.
    topcoat::tailwind::BuildConfig::new().render().unwrap();
}
