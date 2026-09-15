fn main() {
    topcoat::tailwind::BuildConfig::new()
        .input("app.css")
        .render()
        .unwrap();
}
